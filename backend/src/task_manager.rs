use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock};
use tokio::sync::mpsc;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TaskInfo {
    pub task_id: String,
    pub todo_id: i64,
    pub todo_title: String,
    pub executor: String,
    /// 执行记录的日志（JSON 字符串）
    pub logs: String,
}

pub struct TaskManager {
    tasks: Mutex<HashMap<String, mpsc::UnboundedSender<()>>>,
    /// 存储每个任务的基本信息，用于 WebSocket 连接时同步
    task_infos: RwLock<HashMap<String, TaskInfo>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
            task_infos: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, task_id: String) -> mpsc::UnboundedReceiver<()> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.tasks.lock().await.insert(task_id, tx);
        rx
    }

    /// 注册任务信息，用于 WebSocket 同步
    pub async fn register_info(&self, info: TaskInfo) {
        self.task_infos.write().await.insert(info.task_id.clone(), info);
    }

    /// 获取所有当前运行的任务信息
    pub async fn get_all_task_infos(&self) -> Vec<TaskInfo> {
        self.task_infos.read().await.values().cloned().collect()
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
        self.task_infos.write().await.remove(task_id);
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
        assert!(!tm.cancel("task-1").await);
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

    #[tokio::test]
    async fn test_task_info_tracking() {
        let tm = TaskManager::new();
        tm.register_info(TaskInfo {
            task_id: "task-1".to_string(),
            todo_id: 1,
            todo_title: "Test Task".to_string(),
            executor: "claudecode".to_string(),
            logs: "[]".to_string(),
        }).await;
        
        let infos = tm.get_all_task_infos().await;
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].task_id, "task-1");
        
        tm.remove("task-1").await;
        let infos = tm.get_all_task_infos().await;
        assert!(infos.is_empty());
    }
}
