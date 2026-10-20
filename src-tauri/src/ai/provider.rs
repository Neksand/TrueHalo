use super::types::{CompletionRequest, ModelInfo, StreamEventPayload};
use anyhow::Result;
use tokio::sync::{mpsc, oneshot};

#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    fn provider_id(&self) -> &'static str;

    async fn list_models(&self) -> Result<Vec<ModelInfo>>;

    async fn stream(
        &self,
        request: CompletionRequest,
        event_tx: mpsc::Sender<StreamEventPayload>,
        cancel_rx: oneshot::Receiver<()>,
    ) -> Result<()>;
}
