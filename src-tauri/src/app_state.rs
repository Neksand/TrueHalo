use crate::storage::db::DatabaseManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub active_streams: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
    pub db: Arc<DatabaseManager>,
}

impl AppState {
    pub fn new(db: DatabaseManager) -> Self {
        Self {
            active_streams: Arc::new(Mutex::new(HashMap::new())),
            db: Arc::new(db),
        }
    }

    pub async fn register_stream(&self, stream_id: String, cancel_tx: oneshot::Sender<()>) {
        let mut map = self.active_streams.lock().await;
        map.insert(stream_id, cancel_tx);
    }

    pub async fn cancel_stream(&self, stream_id: &str) -> bool {
        let mut map = self.active_streams.lock().await;
        if let Some(tx) = map.remove(stream_id) {
            let _ = tx.send(());
            true
        } else {
            false
        }
    }

    pub async fn remove_stream(&self, stream_id: &str) {
        let mut map = self.active_streams.lock().await;
        map.remove(stream_id);
    }
}
