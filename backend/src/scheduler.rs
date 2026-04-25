use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tokio_cron_scheduler::{Job, JobScheduler};
use log::info;

use crate::adapters::ExecutorRegistry;
use crate::db::Database;
use crate::executor_service::run_todo_execution;
use crate::handlers::ExecEvent;

pub struct TodoScheduler {
    sched: Mutex<JobScheduler>,
}

impl TodoScheduler {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let sched = JobScheduler::new().await?;
        Ok(Self { sched: Mutex::new(sched) })
    }

    pub async fn load_from_db(
        &self,
        db: Arc<Database>,
        executor_registry: Arc<ExecutorRegistry>,
        tx: broadcast::Sender<ExecEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let todos = db.get_scheduler_todos();

        for todo in todos {
            if let Some(ref config) = todo.scheduler_config {
                if todo.scheduler_enabled {
                    info!("Loading scheduled task for todo {} with cron: {}", todo.id, config);
                    self.add_task(
                        db.clone(),
                        executor_registry.clone(),
                        tx.clone(),
                        todo.id,
                        config.clone(),
                    ).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn add_task(
        &self,
        db: Arc<Database>,
        executor_registry: Arc<ExecutorRegistry>,
        tx: broadcast::Sender<ExecEvent>,
        todo_id: i64,
        cron_expr: String,
    ) -> Result<uuid::Uuid, Box<dyn std::error::Error>> {
        let db_clone = db.clone();
        let registry_clone = executor_registry.clone();
        let tx_clone = tx.clone();

        info!("Creating job for todo {} with cron: {}", todo_id, cron_expr);
        let job = Job::new_async(&cron_expr, move |_uuid, _l| {
            let db = db_clone.clone();
            let registry = registry_clone.clone();
            let tx = tx_clone.clone();

            Box::pin(async move {
                if let Some(todo) = db.get_todo(todo_id) {
                    let message = todo.description.clone();
                    let executor = todo.executor.clone();
                    info!("Scheduled execution triggered for todo {}: {}", todo_id, message);
                    run_todo_execution(db, registry, tx, todo_id, message, executor).await;
                }
            })
        })?;

        let job_id = job.guid();
        info!("Job created with guid {}, now adding to scheduler...", job_id);
        let sched = self.sched.lock().await;
        info!("Scheduler inited: {}", sched.inited().await);
        match sched.add(job).await {
            Ok(id) => {
                info!("Added scheduled task {} for todo {} with cron: {}", id, todo_id, cron_expr);
                Ok(id)
            }
            Err(e) => {
                log::error!("Failed to add job to scheduler: {:?}", e);
                Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e))))
            }
        }
    }

    pub async fn remove_task(
        &self,
        job_id: uuid::Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.sched.lock().await.remove(&job_id).await?;
        info!("Removed scheduled task {}", job_id);
        Ok(())
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.sched.lock().await.start().await?;
        info!("Scheduler started");
        Ok(())
    }
}
