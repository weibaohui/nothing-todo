use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

pub struct TaskManager {
    tasks: Mutex<HashMap<String, mpsc::UnboundedSender<()>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
        }
    }

    pub async fn register(&self, task_id: String) -> mpsc::UnboundedReceiver<()> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.tasks.lock().await.insert(task_id, tx);
        rx
    }

    pub async fn cancel(&self, task_id: &str) -> bool {
        if let Some(tx) = self.tasks.lock().await.remove(task_id) {
            let _ = tx.send(());
            true
        } else {
            false
        }
    }

    pub async fn remove(&self, task_id: &str) {
        self.tasks.lock().await.remove(task_id);
    }
}
