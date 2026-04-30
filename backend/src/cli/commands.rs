use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::models::{
    ClientResponse, CreateTagRequest, CreateTodoRequest, DashboardStats, ExecutionRecord,
    ExecutionRecordsPage, ExecutionSummary, Tag, Todo, ExecuteRequest,
};
use crate::cli::client::ApiClient;
use crate::config;

#[derive(Parser, Debug)]
#[command(name = "ntd")]
#[command(about = "AI Todo CLI - Manage AI-powered tasks", long_about = None)]
pub struct Cli {
    /// API server URL (default: from ~/.ntd/config.yaml, or http://localhost:8088)
    #[arg(long)]
    pub server: Option<String>,

    /// Output format
    #[arg(short, long, default_value = "json", value_enum)]
    pub output: OutputFormat,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Json,
    Pretty,
}

// ============== CLI Commands ==============

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Todo management
    Todo {
        #[command(subcommand)]
        action: TodoAction,
    },
    /// Tag management
    Tag {
        #[command(subcommand)]
        action: TagAction,
    },
    /// Global statistics
    Stats,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TodoAction {
    /// Create a new todo
    Create {
        /// Todo title
        title: String,

        /// Prompt content (use --file to load from file)
        #[arg(short, long)]
        prompt: Option<String>,

        /// Read prompt from file
        #[arg(short, long)]
        file: Option<String>,

        /// Executor type (claudecode, joinai, codebuddy, opencode, atomcode, hermes)
        #[arg(short, long)]
        executor: Option<String>,

        /// Working directory
        #[arg(short, long)]
        workspace: Option<String>,

        /// Tag IDs (comma-separated)
        #[arg(long)]
        tags: Option<String>,

        /// Schedule (AI: convert natural language to cron, e.g. "*/30 * * * *")
        #[arg(long)]
        schedule: Option<String>,
    },
    /// List todos
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Filter by tag ID
        #[arg(long)]
        tag: Option<i64>,

        /// Show only running todos
        #[arg(long)]
        running: bool,
    },
    /// Get todo details
    Get {
        /// Todo ID
        id: i64,
    },
    /// Update todo
    Update {
        /// Todo ID
        id: i64,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New prompt (use --file to load from file)
        #[arg(long)]
        prompt: Option<String>,

        /// Read prompt from file
        #[arg(short, long)]
        file: Option<String>,

        /// New status
        #[arg(long)]
        status: Option<String>,

        /// New executor type
        #[arg(long)]
        executor: Option<String>,

        /// New working directory
        #[arg(long)]
        workspace: Option<String>,

        /// New tag IDs (comma-separated)
        #[arg(long)]
        tags: Option<String>,

        /// Schedule (AI: convert natural language to cron)
        #[arg(long)]
        schedule: Option<String>,
    },
    /// Delete todo
    Delete {
        /// Todo ID
        id: i64,
    },
    /// Execute todo
    Execute {
        /// Todo ID
        id: i64,

        /// Additional message
        #[arg(short, long)]
        message: Option<String>,

        /// Override executor
        #[arg(long)]
        executor: Option<String>,
    },
    /// Stop todo execution
    Stop {
        /// Todo ID
        id: i64,
    },
    /// Get todo execution stats
    Stats {
        /// Todo ID
        id: i64,
    },
    /// Execution records
    Execution {
        #[command(subcommand)]
        action: ExecutionAction,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ExecutionAction {
    /// List execution records for a todo
    List {
        /// Todo ID
        todo_id: i64,

        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Page number
        #[arg(long, default_value = "1")]
        page: i64,

        /// Items per page
        #[arg(long, default_value = "20")]
        limit: i64,
    },
    /// Get execution record details
    Get {
        /// Execution record ID
        id: i64,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum TagAction {
    /// List all tags
    List,
    /// Create a new tag
    Create {
        /// Tag name
        name: String,

        /// Tag color (hex)
        #[arg(short, long, default_value = "#1890ff")]
        color: String,
    },
    /// Delete a tag
    Delete {
        /// Tag ID
        id: i64,
    },
}

// ============== Helper Functions ==============

fn read_prompt_from_file(file: &Option<String>) -> anyhow::Result<String> {
    match file {
        Some(path) => Ok(std::fs::read_to_string(path)?),
        None => Ok(String::new()),
    }
}

fn parse_tags(tags: &Option<String>) -> Vec<i64> {
    match tags {
        Some(s) => s.split(',').filter_map(|s| s.trim().parse().ok()).collect(),
        None => Vec::new(),
    }
}

// ============== Main Entry Point ==============

pub async fn run_command(cli: &Cli) -> anyhow::Result<()> {
    let server_url = cli.server.clone().unwrap_or_else(|| config::Config::load().server_url());
    let client = ApiClient::new(&server_url);

    match &cli.command {
        Commands::Todo { action } => handle_todo(&client, action, &cli.output).await?,
        Commands::Tag { action } => handle_tag(&client, action, &cli.output).await?,
        Commands::Stats => handle_stats(&client, &cli.output).await?,
    }

    Ok(())
}

// ============== Todo Handlers ==============

async fn handle_todo(client: &ApiClient, action: &TodoAction, output: &OutputFormat) -> anyhow::Result<()> {
    match action {
        TodoAction::Create { title, prompt, file, executor, workspace: _, tags, schedule: _ } => {
            let prompt_content = if let Some(p) = prompt {
                p.clone()
            } else {
                read_prompt_from_file(file)?
            };

            let req = CreateTodoRequest {
                title: title.clone(),
                prompt: prompt_content,
                tag_ids: parse_tags(tags),
                executor: executor.clone(),
            };

            let resp: ClientResponse<Todo> = client.post("/todos", &req).await?;
            print_response(resp, output);
        }
        TodoAction::List { status, tag, running } => {
            let mut query_params = Vec::new();

            if let Some(s) = status {
                query_params.push(format!("status={}", s));
            }
            if let Some(t) = tag {
                query_params.push(format!("tag_id={}", t));
            }
            if *running {
                query_params.push("running=true".to_string());
            }

            let path = if query_params.is_empty() {
                "/todos".to_string()
            } else {
                format!("/todos?{}", query_params.join("&"))
            };

            let resp: ClientResponse<Vec<Todo>> = client.get(&path).await?;
            print_response(resp, output);
        }
        TodoAction::Get { id } => {
            let resp: ClientResponse<Todo> = client.get(&format!("/todos/{}", id)).await?;
            print_response(resp, output);
        }
	        TodoAction::Update { id, title, prompt, file: _, status, executor, workspace, tags: _, schedule } => {
            let req = serde_json::json!({
                "title": title,
                "prompt": prompt,
                "status": status,
                "executor": executor,
                "workspace": workspace,
                "scheduler_enabled": schedule.as_ref().map(|s| !s.is_empty()),
                "scheduler_config": schedule.clone().filter(|s| !s.is_empty()),
            });
            
            let resp: ClientResponse<Todo> = client.put(&format!("/todos/{}", id), &req).await?;
            print_response(resp, output);
        }

        TodoAction::Delete { id } => {
            let resp: ClientResponse<()> = client.delete(&format!("/todos/{}", id)).await?;
            print_response(resp, output);
        }
        TodoAction::Execute { id, message, executor } => {
            let req = ExecuteRequest {
                todo_id: *id,
                message: message.clone().unwrap_or_default(),
                executor: executor.clone(),
            };
            let resp: ClientResponse<serde_json::Value> = client.post("/execute", &req).await?;
            print_response(resp, output);
        }
        TodoAction::Stop { id } => {
            let req = serde_json::json!({ "todo_id": id });
            let resp: ClientResponse<()> = client.post("/execute/stop", &req).await?;
            print_response(resp, output);
        }
        TodoAction::Stats { id } => {
            let resp: ClientResponse<ExecutionSummary> = client.get(&format!("/todos/{}/summary", id)).await?;
            print_response(resp, output);
        }
        TodoAction::Execution { action } => {
            handle_execution(client, action, output).await?;
        }
    }
    Ok(())
}

async fn handle_execution(client: &ApiClient, action: &ExecutionAction, output: &OutputFormat) -> anyhow::Result<()> {
    match action {
        ExecutionAction::List { todo_id, status, page, limit } => {
            let path = format!(
                "/execution-records?todo_id={}&page={}&limit={}{}",
                todo_id,
                page,
                limit,
                status.as_ref().map(|s| format!("&status={}", s)).unwrap_or_default()
            );
            let resp: ClientResponse<ExecutionRecordsPage> = client.get(&path).await?;
            print_response(resp, output);
        }
        ExecutionAction::Get { id } => {
            let resp: ClientResponse<ExecutionRecord> = client.get(&format!("/execution-records/{}", id)).await?;
            print_response(resp, output);
        }
    }
    Ok(())
}

// ============== Tag Handlers ==============

async fn handle_tag(client: &ApiClient, action: &TagAction, output: &OutputFormat) -> anyhow::Result<()> {
    match action {
        TagAction::List => {
            let resp: ClientResponse<Vec<Tag>> = client.get("/tags").await?;
            print_response(resp, output);
        }
        TagAction::Create { name, color } => {
            let req = CreateTagRequest {
                name: name.clone(),
                color: color.clone(),
            };
            let resp: ClientResponse<Tag> = client.post("/tags", &req).await?;
            print_response(resp, output);
        }
        TagAction::Delete { id } => {
            let resp: ClientResponse<()> = client.delete(&format!("/tags/{}", id)).await?;
            print_response(resp, output);
        }
    }
    Ok(())
}

// ============== Stats Handler ==============

async fn handle_stats(client: &ApiClient, output: &OutputFormat) -> anyhow::Result<()> {
    let resp: ClientResponse<DashboardStats> = client.get("/dashboard-stats").await?;
    print_response(resp, output);
    Ok(())
}

// ============== Output ==============

fn print_response<T: serde::Serialize>(resp: ClientResponse<T>, output: &OutputFormat) {
    match output {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(&resp).unwrap());
        }
        OutputFormat::Pretty => {
            println!("{}", serde_json::to_string_pretty(&resp).unwrap());
        }
    }
}
