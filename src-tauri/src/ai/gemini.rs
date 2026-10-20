use super::provider::AiProvider;
use super::types::{
    ChatMessage, CompletionRequest, ContentPart, MessageRole, ModelInfo, StreamEventPayload,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

pub struct GeminiProvider {
    api_key: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[derive(Deserialize)]
struct GeminiModelsResponse {
    models: Vec<GeminiModelItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiModelItem {
    name: String, // e.g. "models/gemini-1.5-flash"
    display_name: Option<String>,
    description: Option<String>,
    input_token_limit: Option<u64>,
    supported_generation_methods: Option<Vec<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiChatPayload {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<Value>,
}

fn convert_gemini_messages(messages: Vec<ChatMessage>) -> Vec<GeminiContent> {
    messages
        .into_iter()
        .map(|msg| {
            let role = match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "model",
                MessageRole::System => "user",
            }
            .to_string();

            let parts: Vec<Value> = msg
                .content
                .into_iter()
                .map(|part| match part {
                    ContentPart::Text { text } => serde_json::json!({
                        "text": text
                    }),
                    ContentPart::ImageUrl { url } => serde_json::json!({
                        "text": format!("[Attached image URL: {}]", url)
                    }),
                    ContentPart::ImageBase64 { media_type, data } => serde_json::json!({
                        "inline_data": {
                            "mime_type": media_type,
                            "data": data
                        }
                    }),
                })
                .collect();

            GeminiContent { role, parts }
        })
        .collect()
}

#[async_trait]
impl AiProvider for GeminiProvider {
    fn provider_id(&self) -> &'static str {
        "gemini"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models?key={}",
            self.api_key.trim()
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to Google Gemini API")?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini API error: {}", err_text));
        }

        let body: GeminiModelsResponse = resp
            .json()
            .await
            .context("Failed to parse Gemini models response")?;

        let models = body
            .models
            .into_iter()
            .filter(|m| {
                m.supported_generation_methods
                    .as_ref()
                    .map(|methods| methods.iter().any(|method| method == "generateContent"))
                    .unwrap_or(false)
            })
            .map(|item| {
                let id = item.name.trim_start_matches("models/").to_string();
                let name = item.display_name.unwrap_or_else(|| id.clone());
                ModelInfo {
                    id,
                    name,
                    description: item.description,
                    context_length: item.input_token_limit,
                    supports_vision: true, // Modern Gemini models (1.5/2.0) are multimodal natively
                    pricing: None,
                }
            })
            .collect();

        Ok(models)
    }

    async fn stream(
        &self,
        request: CompletionRequest,
        event_tx: mpsc::Sender<StreamEventPayload>,
        mut cancel_rx: oneshot::Receiver<()>,
    ) -> Result<()> {
        let clean_model = request.model.trim_start_matches("models/");
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            clean_model,
            self.api_key.trim()
        );

        let payload = GeminiChatPayload {
            contents: convert_gemini_messages(request.messages),
            generation_config: if request.temperature.is_some() || request.max_tokens.is_some() {
                Some(GeminiGenerationConfig {
                    temperature: request.temperature,
                    max_output_tokens: request.max_tokens,
                })
            } else {
                None
            },
        };

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let resp = self
            .client
            .post(&url)
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            let err_msg = format!("Gemini HTTP {}: {}", status, err_body);
            let _ = event_tx
                .send(StreamEventPayload::Error {
                    message: err_msg.clone(),
                })
                .await;
            return Err(anyhow!(err_msg));
        }

        let mut stream = resp.bytes_stream().eventsource();

        loop {
            tokio::select! {
                _ = &mut cancel_rx => {
                    tracing::info!("Gemini stream cancelled by user");
                    let _ = event_tx.send(StreamEventPayload::Done {
                        prompt_tokens: None,
                        completion_tokens: None,
                        total_tokens: None,
                    }).await;
                    return Ok(());
                }
                event_opt = stream.next() => {
                    match event_opt {
                        Some(Ok(event)) => {
                            if let Ok(parsed) = serde_json::from_str::<Value>(&event.data) {
                                if let Some(candidates) = parsed.get("candidates").and_then(|c| c.as_array()) {
                                    if let Some(candidate) = candidates.first() {
                                        if let Some(parts) = candidate["content"]["parts"].as_array() {
                                            for part in parts {
                                                if let Some(text) = part["text"].as_str() {
                                                    if !text.is_empty() {
                                                        let _ = event_tx.send(StreamEventPayload::Delta {
                                                            text: text.to_string(),
                                                        }).await;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Some(usage) = parsed.get("usageMetadata") {
                                    let prompt_tokens = usage["promptTokenCount"].as_u64().map(|v| v as u32);
                                    let completion_tokens = usage["candidatesTokenCount"].as_u64().map(|v| v as u32);
                                    let total_tokens = usage["totalTokenCount"].as_u64().map(|v| v as u32);

                                    if prompt_tokens.is_some() || completion_tokens.is_some() {
                                        let _ = event_tx.send(StreamEventPayload::Done {
                                            prompt_tokens,
                                            completion_tokens,
                                            total_tokens,
                                        }).await;
                                        break;
                                    }
                                }
                            }
                        }
                        Some(Err(err)) => {
                            let _ = event_tx.send(StreamEventPayload::Error {
                                message: format!("Gemini stream parsing error: {}", err),
                            }).await;
                            break;
                        }
                        None => {
                            let _ = event_tx.send(StreamEventPayload::Done {
                                prompt_tokens: None,
                                completion_tokens: None,
                                total_tokens: None,
                            }).await;
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
