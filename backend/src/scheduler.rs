use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tokio_cron_scheduler::{Job, JobScheduler};
use log::info;

use crate::adapters::ExecutorRegistry;
use crate::db::Database;
use crate::executor_service::run_todo_execution;
use crate::handlers::ExecEvent;
use crate::task_manager::TaskManager;

pub struct TodoScheduler {
    sched: Mutex<JobScheduler>,
    job_map: Mutex<HashMap<i64, uuid::Uuid>>,
}

impl TodoScheduler {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let sched = JobScheduler::new().await?;
        Ok(Self {
            sched: Mutex::new(sched),
            job_map: Mutex::new(HashMap::new()),
        })
    }

    pub async fn load_from_db(
        &self,
        db: Arc<Database>,
        executor_registry: Arc<ExecutorRegistry>,
        tx: broadcast::Sender<ExecEvent>,
        task_manager: Arc<TaskManager>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let todos = db.get_scheduler_todos();

        for todo in todos {
            if let Some(ref config) = todo.scheduler_config {
                if todo.scheduler_enabled {
                    info!("Loading scheduled task for todo {} with cron: {}", todo.id, config);
                    self.upsert_task(
                        db.clone(),
                        executor_registry.clone(),
                        tx.clone(),
                        todo.id,
                        config.clone(),
                        task_manager.clone(),
                    ).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn upsert_task(
        &self,
        db: Arc<Database>,
        executor_registry: Arc<ExecutorRegistry>,
        tx: broadcast::Sender<ExecEvent>,
        todo_id: i64,
        cron_expr: String,
        task_manager: Arc<TaskManager>,
    ) -> Result<uuid::Uuid, Box<dyn std::error::Error>> {
        self.remove_task_for_todo(todo_id).await;

        let db_clone = db.clone();
        let registry_clone = executor_registry.clone();
        let tx_clone = tx.clone();
        let tm_clone = task_manager.clone();

        info!("Creating job for todo {} with cron: {}", todo_id, cron_expr);
        let job = Job::new_async(&cron_expr, move |_uuid, _l| {
            let db = db_clone.clone();
            let registry = registry_clone.clone();
            let tx = tx_clone.clone();
            let tm = tm_clone.clone();

            Box::pin(async move {
                if let Some(todo) = db.get_todo(todo_id) {
                    let message = if todo.prompt.is_empty() { todo.title.clone() } else { todo.prompt.clone() };
                    let executor = todo.executor.clone();
                    info!("Scheduled execution triggered for todo {}: {}", todo_id, message);
                    run_todo_execution(db, registry, tx, todo_id, message, executor, "cron", tm).await;
                }
            })
        })?;

        let job_id = job.guid();
        info!("Job created with guid {}, now adding to scheduler...", job_id);
        let sched = self.sched.lock().await;
        info!("Scheduler inited: {}", sched.inited().await);
        match sched.add(job).await {
            Ok(id) => {
                drop(sched);
                self.job_map.lock().await.insert(todo_id, id);
                info!("Added scheduled task {} for todo {} with cron: {}", id, todo_id, cron_expr);
                Ok(id)
            }
            Err(e) => {
                log::error!("Failed to add job to scheduler: {:?}", e);
                Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e))))
            }
        }
    }

    pub async fn remove_task_for_todo(&self, todo_id: i64) {
        let job_id = self.job_map.lock().await.remove(&todo_id);
        if let Some(job_id) = job_id {
            match self.sched.lock().await.remove(&job_id).await {
                Ok(_) => info!("Removed scheduled task {} for todo {}", job_id, todo_id),
                Err(e) => log::error!("Failed to remove scheduled task {} for todo {}: {:?}", job_id, todo_id, e),
            }
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.sched.lock().await.start().await?;
        info!("Scheduler started");
        Ok(())
    }
}
