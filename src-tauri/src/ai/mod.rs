pub mod gemini;
pub mod local;
pub mod openai;
pub mod openrouter;
pub mod provider;
pub mod types;

use anyhow::{anyhow, Result};
use gemini::GeminiProvider;
use local::LocalProvider;
use openai::OpenAiProvider;
use openrouter::OpenRouterProvider;
use provider::AiProvider;

pub fn get_provider(provider_id: &str, api_key: String) -> Result<Box<dyn AiProvider>> {
    match provider_id.to_lowercase().as_str() {
        "openrouter" => Ok(Box::new(OpenRouterProvider::new(api_key))),
        "gemini" | "google" => Ok(Box::new(GeminiProvider::new(api_key))),
        "openai" => Ok(Box::new(OpenAiProvider::new(api_key))),
        other => Err(anyhow!("Unsupported AI provider: {}", other)),
    }
}

/// Local servers need an endpoint, not just a key.
pub fn get_local_provider(base_url: String, api_key: String) -> Box<dyn AiProvider> {
    Box::new(LocalProvider::new(base_url, api_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::{ChatMessage, CompletionRequest, ContentPart, MessageRole, StreamEventPayload};

    #[test]
    fn test_provider_factory() {
        let p1 = get_provider("openrouter", "test-key".into()).unwrap();
        assert_eq!(p1.provider_id(), "openrouter");

        let p2 = get_provider("gemini", "test-key".into()).unwrap();
        assert_eq!(p2.provider_id(), "gemini");

        let p3 = get_provider("openai", "test-key".into()).unwrap();
        assert_eq!(p3.provider_id(), "openai");

        assert!(get_provider("unknown_provider", "key".into()).is_err());
    }

    #[test]
    fn test_stream_event_payload_serialization() {
        let delta = StreamEventPayload::Delta {
            text: "Hello from TrueHalo".to_string(),
        };
        let serialized = serde_json::to_string(&delta).unwrap();
        assert!(serialized.contains(r#""type":"delta""#));
        assert!(serialized.contains("Hello from TrueHalo"));

        let done = StreamEventPayload::Done {
            prompt_tokens: Some(10),
            completion_tokens: Some(25),
            total_tokens: Some(35),
        };
        let done_serialized = serde_json::to_string(&done).unwrap();
        assert!(done_serialized.contains(r#""type":"done""#));
        assert!(done_serialized.contains(r#""total_tokens":35"#));
    }

    #[test]
    fn test_chat_message_creation() {
        let user_msg = ChatMessage::user_text("What is TrueHalo?");
        assert_eq!(user_msg.role, MessageRole::User);
        assert_eq!(user_msg.content.len(), 1);

        let assistant_msg = ChatMessage::assistant_text("TrueHalo is an AI overlay.");
        assert_eq!(assistant_msg.role, MessageRole::Assistant);
        assert_eq!(assistant_msg.content.len(), 1);
    }

    #[test]
    fn test_multimodal_message_support() {
        let multimodal = ChatMessage {
            role: MessageRole::User,
            content: vec![
                ContentPart::Text {
                    text: "Analyze this screen".to_string(),
                },
                ContentPart::ImageBase64 {
                    media_type: "image/png".to_string(),
                    data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=".to_string(),
                },
            ],
        };
        assert_eq!(multimodal.content.len(), 2);

        let req = CompletionRequest {
            stream_id: "req-123".into(),
            provider: "openrouter".into(),
            model: "anthropic/claude-3.5-sonnet".into(),
            messages: vec![multimodal],
            temperature: Some(0.7),
            max_tokens: Some(1024),
        };

        let req_json = serde_json::to_string(&req).unwrap();
        assert!(req_json.contains("anthropic/claude-3.5-sonnet"));
        assert!(req_json.contains("Analyze this screen"));
    }
}
