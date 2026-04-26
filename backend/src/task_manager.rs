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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_creates_receiver() {
        let tm = TaskManager::new();
        let mut rx = tm.register("task-1".to_string()).await;
        // cancel should signal the receiver
        tm.cancel("task-1").await;
        assert!(rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn test_cancel_returns_true_when_found() {
        let tm = TaskManager::new();
        let _rx = tm.register("task-1".to_string()).await;
        assert!(tm.cancel("task-1").await);
    }

    #[tokio::test]
    async fn test_cancel_returns_false_when_not_found() {
        let tm = TaskManager::new();
        assert!(!tm.cancel("nonexistent").await);
    }

    #[tokio::test]
    async fn test_remove_cleans_up() {
        let tm = TaskManager::new();
        let _rx = tm.register("task-1".to_string()).await;
        tm.remove("task-1").await;
        assert!(!tm.cancel("task-1").await);
    }

    #[tokio::test]
    async fn test_multiple_tasks_independent() {
        let tm = TaskManager::new();
        let _rx1 = tm.register("task-1".to_string()).await;
        let _rx2 = tm.register("task-2".to_string()).await;

        assert!(tm.cancel("task-1").await);
        assert!(tm.cancel("task-2").await);
        assert!(!tm.cancel("task-1").await); // already removed
    }
}
