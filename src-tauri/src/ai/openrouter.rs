use super::provider::AiProvider;
use super::types::{
    ChatMessage, CompletionRequest, ContentPart, MessageRole, ModelInfo, ModelPricing,
    StreamEventPayload,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

pub struct OpenRouterProvider {
    api_key: String,
    client: reqwest::Client,
}

impl OpenRouterProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let auth_val = format!("Bearer {}", self.api_key.trim());
        if let Ok(val) = HeaderValue::from_str(&auth_val) {
            headers.insert(AUTHORIZATION, val);
        }
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "HTTP-Referer",
            HeaderValue::from_static("https://truehalo.app"),
        );
        headers.insert("X-Title", HeaderValue::from_static("TrueHalo"));
        headers
    }
}

#[derive(Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModelItem>,
}

#[derive(Deserialize)]
struct OpenRouterModelItem {
    id: String,
    name: Option<String>,
    description: Option<String>,
    context_length: Option<u64>,
    architecture: Option<OpenRouterArchitecture>,
    pricing: Option<OpenRouterPricing>,
}

#[derive(Deserialize)]
struct OpenRouterArchitecture {
    modality: Option<String>,
}

#[derive(Deserialize)]
struct OpenRouterPricing {
    prompt: Option<String>,
    completion: Option<String>,
}

#[derive(Serialize)]
struct OpenRouterChatPayload {
    model: String,
    messages: Vec<OpenRouterMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct OpenRouterMessage {
    role: String,
    content: Value,
}

fn convert_messages(messages: Vec<ChatMessage>) -> Vec<OpenRouterMessage> {
    messages
        .into_iter()
        .map(|msg| {
            let role_str = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            }
            .to_string();

            // Check if there are multimodal parts
            let has_images = msg.content.iter().any(|p| match p {
                ContentPart::Text { .. } => false,
                ContentPart::ImageUrl { .. } | ContentPart::ImageBase64 { .. } => true,
            });

            let content_value = if !has_images {
                let full_text = msg
                    .content
                    .into_iter()
                    .filter_map(|p| match p {
                        ContentPart::Text { text } => Some(text),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Value::String(full_text)
            } else {
                let parts: Vec<Value> = msg
                    .content
                    .into_iter()
                    .map(|part| match part {
                        ContentPart::Text { text } => serde_json::json!({
                            "type": "text",
                            "text": text
                        }),
                        ContentPart::ImageUrl { url } => serde_json::json!({
                            "type": "image_url",
                            "image_url": { "url": url }
                        }),
                        ContentPart::ImageBase64 { media_type, data } => serde_json::json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};base64,{}", media_type, data)
                            }
                        }),
                    })
                    .collect();
                Value::Array(parts)
            };

            OpenRouterMessage {
                role: role_str,
                content: content_value,
            }
        })
        .collect()
}

#[async_trait]
impl AiProvider for OpenRouterProvider {
    fn provider_id(&self) -> &'static str {
        "openrouter"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let resp = self
            .client
            .get("https://openrouter.ai/api/v1/models")
            .headers(self.headers())
            .send()
            .await
            .context("Failed to connect to OpenRouter")?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("OpenRouter error: {}", err_text));
        }

        let body: OpenRouterModelsResponse = resp
            .json()
            .await
            .context("Failed to parse OpenRouter models response")?;

        let models = body
            .data
            .into_iter()
            .map(|item| {
                let supports_vision = item
                    .architecture
                    .and_then(|a| a.modality)
                    .map(|m| m.contains("image"))
                    .unwrap_or(false);

                let pricing = item.pricing.map(|p| ModelPricing {
                    prompt: p.prompt,
                    completion: p.completion,
                });

                ModelInfo {
                    name: item.name.unwrap_or_else(|| item.id.clone()),
                    id: item.id,
                    description: item.description,
                    context_length: item.context_length,
                    supports_vision,
                    pricing,
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
        let payload = OpenRouterChatPayload {
            model: request.model,
            messages: convert_messages(request.messages),
            stream: true,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let resp = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .headers(self.headers())
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to OpenRouter")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            let err_msg = format!("OpenRouter HTTP {}: {}", status, err_body);
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
                    tracing::info!("OpenRouter generation cancelled by user");
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
                            if event.data == "[DONE]" {
                                let _ = event_tx.send(StreamEventPayload::Done {
                                    prompt_tokens: None,
                                    completion_tokens: None,
                                    total_tokens: None,
                                }).await;
                                break;
                            }

                            if let Ok(parsed) = serde_json::from_str::<Value>(&event.data) {
                                if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                                    if !content.is_empty() {
                                        let _ = event_tx.send(StreamEventPayload::Delta {
                                            text: content.to_string(),
                                        }).await;
                                    }
                                }

                                // Check for finish reason or usage
                                if let Some(usage) = parsed.get("usage") {
                                    let prompt_tokens = usage["prompt_tokens"].as_u64().map(|v| v as u32);
                                    let completion_tokens = usage["completion_tokens"].as_u64().map(|v| v as u32);
                                    let total_tokens = usage["total_tokens"].as_u64().map(|v| v as u32);

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
                                message: format!("Stream decoding error: {}", err),
                            }).await;
                            break;
                        }
                        None => {
                            // Stream closed normally
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
