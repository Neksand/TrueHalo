use super::provider::AiProvider;
use super::types::{CompletionRequest, ModelInfo, StreamEventPayload};
use anyhow::{Context, Result};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

/// OpenAI-compatible local server: Ollama, LM Studio, llama.cpp, etc.
///
/// The default endpoint is Ollama (`http://localhost:11434/v1`); the user can
/// point it at any OpenAI-compatible base URL in Settings. No API key is
/// required, so nothing is read from the credential store for this provider.
pub struct LocalProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl LocalProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url: base_url.trim().trim_end_matches('/').to_string(),
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_default(),
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if !self.api_key.trim().is_empty() {
            let auth_val = format!("Bearer {}", self.api_key.trim());
            if let Ok(val) = HeaderValue::from_str(&auth_val) {
                headers.insert(AUTHORIZATION, val);
            }
        }
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }

    fn models_url(&self) -> String {
        format!("{}/models", self.base_url)
    }

    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }
}

#[derive(Deserialize)]
struct LocalModelsResponse {
    data: Option<Vec<LocalModelItem>>,
}

#[derive(Deserialize)]
struct LocalModelItem {
    id: Option<String>,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Option<Vec<OllamaTagItem>>,
}

#[derive(Deserialize)]
struct OllamaTagItem {
    name: Option<String>,
}

#[derive(Serialize)]
struct LocalChatPayload {
    model: String,
    messages: Vec<LocalMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct LocalMessage {
    role: String,
    content: Value,
}

fn convert_local_messages(messages: Vec<super::types::ChatMessage>) -> Vec<LocalMessage> {
    use super::types::{ContentPart, MessageRole};
    messages
        .into_iter()
        .map(|msg| {
            let role_str = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            }
            .to_string();

            let has_images = msg.content.iter().any(|p| match p {
                ContentPart::Text { .. } => false,
                ContentPart::ImageUrl { .. } | ContentPart::ImageBase64 { .. } => true,
            });

            let content_value = if !has_images {
                let text = msg
                    .content
                    .into_iter()
                    .filter_map(|p| match p {
                        ContentPart::Text { text } => Some(text),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Value::String(text)
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

            LocalMessage {
                role: role_str,
                content: content_value,
            }
        })
        .collect()
}

fn looks_like_vision_model(id: &str) -> bool {
    let lower = id.to_lowercase();
    ["vision", "llava", "moondream", "minicpm", "qwen2-vl", "qwen-vl", "bakllava", "obsidian"]
        .iter()
        .any(|hint| lower.contains(hint))
}

#[async_trait]
impl AiProvider for LocalProvider {
    fn provider_id(&self) -> &'static str {
        "local"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        // Standard OpenAI-compatible endpoint first (`/v1/models`).
        let resp = self
            .client
            .get(self.models_url())
            .headers(self.headers())
            .send()
            .await
            .context("Failed to connect to the local server")?;

        if resp.status().is_success() {
            if let Ok(body) = resp.json::<LocalModelsResponse>().await {
                if let Some(data) = body.data {
                    let models = data
                        .into_iter()
                        .filter_map(|m| m.id)
                        .map(|id| ModelInfo {
                            name: id.clone(),
                            id: id.clone(),
                            description: None,
                            context_length: None,
                            supports_vision: looks_like_vision_model(&id),
                            pricing: None,
                        })
                        .collect::<Vec<_>>();
                    if !models.is_empty() {
                        return Ok(models);
                    }
                }
            }
        }

        // Ollama native fallback (`/api/tags`).
        let tags_url = self
            .base_url
            .trim_end_matches("/v1")
            .trim_end_matches('/')
            .to_string()
            + "/api/tags";
        let resp = self
            .client
            .get(&tags_url)
            .send()
            .await
            .context("Failed to connect to the local server")?;
        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Local server error: {}", err_text));
        }
        let body: OllamaTagsResponse = resp
            .json()
            .await
            .context("Failed to parse the local model list")?;
        Ok(body
            .models
            .unwrap_or_default()
            .into_iter()
            .filter_map(|m| m.name)
            .map(|id| ModelInfo {
                name: id.clone(),
                id: id.clone(),
                description: None,
                context_length: None,
                supports_vision: looks_like_vision_model(&id),
                pricing: None,
            })
            .collect())
    }

    async fn stream(
        &self,
        request: CompletionRequest,
        event_tx: mpsc::Sender<StreamEventPayload>,
        mut cancel_rx: oneshot::Receiver<()>,
    ) -> Result<()> {
        let payload = LocalChatPayload {
            model: request.model,
            messages: convert_local_messages(request.messages),
            stream: true,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let resp = self
            .client
            .post(self.chat_url())
            .headers(self.headers())
            .json(&payload)
            .send()
            .await
            .context("Failed to connect to the local server")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            let err_msg = format!("Local server HTTP {}: {}", status, err_body);
            let _ = event_tx
                .send(StreamEventPayload::Error {
                    message: err_msg.clone(),
                })
                .await;
            return Err(anyhow::anyhow!(err_msg));
        }

        let mut stream = resp.bytes_stream().eventsource();

        loop {
            tokio::select! {
                _ = &mut cancel_rx => {
                    tracing::info!("Local generation cancelled");
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
                                // Local servers often report no usage; tolerate that.
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
                        Some(Err(e)) => {
                            tracing::warn!("Local stream error: {}", e);
                            break;
                        }
                        None => break,
                    }
                }
            }
        }

        Ok(())
    }
}
