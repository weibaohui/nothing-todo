use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
use ntd::{adapters, db, handlers, scheduler::TodoScheduler, task_manager::TaskManager};

/// Tests must run serially because multiple concurrent opencode processes
/// compete for resources and can cause timeouts.
static TEST_MUTEX: std::sync::LazyLock<Mutex<()>> = std::sync::LazyLock::new(|| Mutex::new(()));

struct TestServer {
    base_url: String,
    _server: tokio::task::JoinHandle<()>,
    _temp_dir: tempfile::TempDir,
}

async fn start_test_server() -> TestServer {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = Arc::new(db::Database::new(db_path.to_str().unwrap()).await.unwrap());

    let executor_registry = Arc::new(adapters::ExecutorRegistry::new());
    executor_registry.register(adapters::joinai::JoinaiExecutor::new("joinai".to_string()));
    executor_registry.register(adapters::claude_code::ClaudeCodeExecutor::new("claude".to_string()));
    executor_registry.register(adapters::codebuddy::CodebuddyExecutor::new("codebuddy".to_string()));
    executor_registry.register(adapters::opencode::OpencodeExecutor::new("opencode".to_string()));

    let (tx, _rx) = broadcast::channel(100);
    let task_manager = Arc::new(TaskManager::new());

    let scheduler = Arc::new(TodoScheduler::new().await.unwrap());
    scheduler
        .load_from_db(db.clone(), executor_registry.clone(), tx.clone(), task_manager.clone())
        .await
        .unwrap();
    scheduler.start().await.unwrap();

    let app = handlers::create_app(db, executor_registry, tx, scheduler, task_manager);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    TestServer {
        base_url: format!("http://127.0.0.1:{}", port),
        _server: server,
        _temp_dir: temp_dir,
    }
}

fn count_opencode_processes(msg: &str) -> usize {
    let output = std::process::Command::new("pgrep")
        .args(["-f", &format!("opencode run.*{}", msg)])
        .output()
        .unwrap();
    if output.status.success() {
        String::from_utf8_lossy(&output.stdout).lines().count()
    } else {
        0
    }
}

fn kill_opencode_processes(msg: &str) {
    let _ = std::process::Command::new("pkill")
        .args(["-f", &format!("opencode run.*{}", msg)])
        .output();
}

async fn create_test_todo(base_url: &str) -> i64 {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/xyz/todos", base_url))
        .json(&serde_json::json!({
            "title": "test",
            "prompt": "echo hello",
            "tag_ids": []
        }))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();
    body["data"]["id"].as_i64().unwrap()
}

#[tokio::test]
async fn test_manual_stop_cleans_up_processes() {
    let _guard = TEST_MUTEX.lock().await;
    let server = start_test_server().await;
    let todo_id = create_test_todo(&server.base_url).await;
    let client = reqwest::Client::new();

    let unique_msg = "test-manual-stop";

    kill_opencode_processes(unique_msg);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let before = count_opencode_processes(unique_msg);

    let exec_resp = client
        .post(format!("{}/xyz/execute", server.base_url))
        .json(&serde_json::json!({
            "todo_id": todo_id,
            "message": unique_msg,
            "executor": "opencode"
        }))
        .send()
        .await
        .unwrap();

    let exec_body: serde_json::Value = exec_resp.json().await.unwrap();
    assert_eq!(exec_body["code"], 0, "execute should succeed");
    let task_id = exec_body["data"]["task_id"].as_str().unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;
    let during = count_opencode_processes(unique_msg);
    assert!(
        during > before,
        "expected opencode processes to be running, got {} (before: {})",
        during,
        before
    );

    let stop_resp = client
        .post(format!("{}/xyz/execute/stop", server.base_url))
        .json(&serde_json::json!({ "task_id": task_id }))
        .send()
        .await
        .unwrap();

    let stop_body: serde_json::Value = stop_resp.json().await.unwrap();
    assert_eq!(stop_body["code"], 0, "stop should succeed");

    tokio::time::sleep(Duration::from_secs(2)).await;
    let after = count_opencode_processes(unique_msg);
    assert_eq!(
        after, before,
        "expected all opencode processes to be cleaned up after stop, got {} (before: {})",
        after, before
    );
}

#[tokio::test]
async fn test_natural_completion_cleans_up_processes() {
    let _guard = TEST_MUTEX.lock().await;
    let server = start_test_server().await;
    let todo_id = create_test_todo(&server.base_url).await;
    let client = reqwest::Client::new();

    let unique_msg = "test-natural-completion";

    kill_opencode_processes(unique_msg);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let before = count_opencode_processes(unique_msg);

    let exec_resp = client
        .post(format!("{}/xyz/execute", server.base_url))
        .json(&serde_json::json!({
            "todo_id": todo_id,
            "message": unique_msg,
            "executor": "opencode"
        }))
        .send()
        .await
        .unwrap();

    let exec_body: serde_json::Value = exec_resp.json().await.unwrap();
    assert_eq!(exec_body["code"], 0, "execute should succeed");

    tokio::time::sleep(Duration::from_secs(2)).await;
    let during = count_opencode_processes(unique_msg);
    assert!(
        during > before,
        "expected opencode processes to be running, got {} (before: {})",
        during,
        before
    );

    let mut after = during;
    for _ in 0..60 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        after = count_opencode_processes(unique_msg);
        if after == before {
            break;
        }
    }

    assert_eq!(
        after, before,
        "expected all opencode processes to be cleaned up after natural completion, got {} (before: {})",
        after, before
    );
}
