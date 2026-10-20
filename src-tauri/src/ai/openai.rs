use super::provider::AiProvider;
use super::types::{
    ChatMessage, CompletionRequest, ContentPart, MessageRole, ModelInfo, StreamEventPayload,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

pub struct OpenAiProvider {
    api_key: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
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
        headers
    }
}

#[derive(Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModelItem>,
}

#[derive(Deserialize)]
struct OpenAiModelItem {
    id: String,
}

#[derive(Serialize)]
struct OpenAiChatPayload {
    model: String,
    messages: Vec<OpenAiMessage>,
    stream: bool,
    stream_options: OpenAiStreamOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct OpenAiStreamOptions {
    include_usage: bool,
}

#[derive(Serialize)]
struct OpenAiMessage {
    role: String,
    content: Value,
}

fn convert_openai_messages(messages: Vec<ChatMessage>) -> Vec<OpenAiMessage> {
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

            OpenAiMessage {
                role: role_str,
                content: content_value,
            }
        })
        .collect()
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    fn provider_id(&self) -> &'static str {
        "openai"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let resp = self
            .client
            .get("https://api.openai.com/v1/models")
            .headers(self.headers())
            .send()
            .await
            .context("Failed to connect to OpenAI")?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI API error: {}", err_text));
        }

        let body: OpenAiModelsResponse = resp
            .json()
            .await
            .context("Failed to parse OpenAI models response")?;

        let filtered_models = body
            .data
            .into_iter()
            .filter(|m| {
                m.id.starts_with("gpt-") || m.id.starts_with("o1") || m.id.starts_with("chatgpt")
            })
            .map(|item| {
                let supports_vision =
                    item.id.contains("4o") || item.id.contains("vision") || item.id.contains("o1");
                ModelInfo {
                    id: item.id.clone(),
                    name: item.id,
                    description: None,
                    context_length: Some(128_000),
                    supports_vision,
                    pricing: None,
                }
            })
            .collect();

        Ok(filtered_models)
    }

    async fn stream(
        &self,
        request: CompletionRequest,
        event_tx: mpsc::Sender<StreamEventPayload>,
        mut cancel_rx: oneshot::Receiver<()>,
    ) -> Result<()> {
        let payload = OpenAiChatPayload {
            model: request.model,
            messages: convert_openai_messages(request.messages),
            stream: true,
            stream_options: OpenAiStreamOptions {
                include_usage: true,
            },
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .headers(self.headers())
            .json(&payload)
            .send()
            .await
            .context("Failed to connect to OpenAI")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            let err_msg = format!("OpenAI HTTP {}: {}", status, err_body);
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
                    tracing::info!("OpenAI generation cancelled");
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
                            let _ = event_tx.send(StreamEventPayload::Error {
                                message: format!("OpenAI stream error: {}", e),
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
