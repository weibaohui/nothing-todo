//! Claude Code CLI executor 适配。
//!
//! # 与 cc-connect 的对应
//!
//! 对应 `cc-connect/agent/claudecode/`（1502+1275 行）。本文件是
//! v1 最小可用版本：spawn 子进程 + stdin/stdout NDJSON + 关键事件。
//!
//! # 协议
//!
//! Claude Code CLI 通过 `--output-format stream-json --input-format stream-json`
//! 启用 NDJSON 双向通信。
//!
//! ## Claude → Dispatcher（stdout 每行一个 JSON）
//!
//! - `{"type":"system","subtype":"init",...}` → ignored
//! - `{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"..."}]}}` → `Event::Text`
//! - `{"type":"assistant","message":{...,"content":[{"type":"tool_use",...}]}}` → `Event::ToolUse`
//! - `{"type":"user","message":{...}}`（replay-user-messages 回放）→ ignored
//! - `{"type":"result","subtype":"success|error","duration_ms":N,"usage":{...}}` → `Event::Result` 或 `Event::Error`
//! - `{"type":"control_request","request_id":"...","request":{"subtype":"can_use_tool",...}}` → `Event::PermissionRequest`
//!
//! ## Dispatcher → Claude（stdin 每行一个 JSON）
//!
//! - `{"type":"user","message":{"role":"user","content":[{"type":"text","text":"..."}]}}`
//! - `{"type":"control_response","response":{"request_id":"...","behavior":"allow|deny"}}`

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex};

use crate::agent::{Agent, AgentSession, Event};
use crate::error::{Error, Result};
use crate::types::{AgentContext, AgentSessionInfo, Attachment, PermissionResult, Usage};

/// ClaudeCodeAgent：项目级配置（claude binary 路径、默认 args）。
///
/// Agent 实例本身无状态；每次 [`start_session`](Agent::start_session) spawn
/// 一个独立的子进程。
#[derive(Clone)]
pub struct ClaudeCodeAgent {
    /// claude 二进制路径。生产用 `/usr/local/bin/claude` 或 PATH 查找；
    /// 测试用 mock shell 脚本路径。
    pub claude_path: PathBuf,
    /// 默认 CLI 参数（v1 写死 stream-json + stdio permission prompt）。
    pub default_args: Vec<String>,
}

impl ClaudeCodeAgent {
    /// 用默认配置构造（claude binary 走 PATH）。
    pub fn new() -> Self {
        Self {
            claude_path: PathBuf::from("claude"),
            default_args: vec![
                "--output-format".into(),
                "stream-json".into(),
                "--input-format".into(),
                "stream-json".into(),
                "--permission-prompt-tool".into(),
                "stdio".into(),
                "--replay-user-messages".into(),
                "--verbose".into(),
            ],
        }
    }

    /// 指定 binary 路径（测试用）。
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            claude_path: path.into(),
            ..Self::new()
        }
    }
}

impl Default for ClaudeCodeAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for ClaudeCodeAgent {
    fn name(&self) -> &'static str {
        "claude-code"
    }

    async fn start_session(
        &self,
        ctx: &AgentContext,
        session_id: Option<&str>,
    ) -> Result<Box<dyn AgentSession>> {
        // 1. 构造 command + args
        let mut cmd = Command::new(&self.claude_path);
        cmd.args(&self.default_args);
        if let Some(sid) = session_id {
            cmd.arg("--resume").arg(sid);
        }
        // 2. stdio pipeline + 不接管 console
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true); // drop session 时杀子进程

        // 3. 设 working dir（Claude Code 在 work_dir 下读 .claude/ 配置）
        if let Some(work_dir) = &ctx.work_dir {
            cmd.current_dir(work_dir);
        }

        // 4. spawn
        let mut child: Child = cmd
            .spawn()
            .map_err(|e| Error::agent(format!("claude spawn failed: {e}")))?;

        // 5. 取 stdio handle
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::agent("claude stdin missing".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::agent("claude stdout missing".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::agent("claude stderr missing".to_string()))?;

        // 6. 构造 session id（spawn 成功就给 uuid；resume 时用传入的 id）
        let session_id_owned = session_id
            .map(String::from)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // 7. 启动 stderr drain task（避免 pipe buffer 阻塞；忽略内容，仅 warn）
        tokio::spawn(drain_stderr(session_id_owned.clone(), stderr));

        // 8. 启动 stdout read loop → mpsc::Sender<Event>
        let alive = Arc::new(AtomicBool::new(true));
        let (tx, rx) = mpsc::channel::<Event>(64);
        let alive_for_reader = alive.clone();
        let session_id_for_reader = session_id_owned.clone();
        tokio::spawn(async move {
            read_stdout_loop(session_id_for_reader, stdout, tx, alive_for_reader).await;
        });

        // 9. 启动 wait task（等子进程退出 → alive=false）
        let alive_for_wait = alive.clone();
        tokio::spawn(async move {
            // 等子进程结束（不管成功失败），标记 alive=false；
            // reader task 的 stream 在 tx drop 后会收到 channel close 事件。
            let _ = child.wait().await;
            alive_for_wait.store(false, Ordering::Release);
            // child drop 在这里发生；stdin/stdout 已 take，drop 仅释放句柄。
        });

        Ok(Box::new(ClaudeCodeSession {
            session_id: session_id_owned,
            stdin: Mutex::new(Some(stdin)),
            alive,
            events_rx: StdMutex::new(Some(rx)),
        }))
    }

    async fn list_sessions(&self, _ctx: &AgentContext) -> Result<Vec<AgentSessionInfo>> {
        // v1: 不持久化 session 列表，调用方用 `--resume <id>` 显式 resume。
        Ok(Vec::new())
    }

    async fn stop(&self) -> Result<()> {
        // v1: 单进程无后台 session（每次 start_session spawn 后移交所有权）。
        Ok(())
    }
}

/// drain 子进程 stderr 到 tracing（避免 pipe buffer 阻塞子进程）。
async fn drain_stderr(session_id: String, mut stderr: tokio::process::ChildStderr) {
    let mut reader = BufReader::new(&mut stderr);
    let mut line = String::new();
    while let Ok(n) = reader.read_line(&mut line).await {
        if n == 0 {
            break;
        }
        if !line.trim().is_empty() {
            tracing::warn!("[claude:{}] stderr: {}", session_id, line.trim());
        }
        line.clear();
    }
}

/// stdout NDJSON → Event 主循环。
///
/// 每行一个 JSON。读出错或 channel close → 退出（reader task 结束，
/// dispatcher 端 recv() 会拿到 None）。
async fn read_stdout_loop(
    session_id: String,
    stdout: tokio::process::ChildStdout,
    tx: mpsc::Sender<Event>,
    alive: Arc<AtomicBool>,
) {
    let mut reader = BufReader::new(stdout).lines();
    loop {
        match reader.next_line().await {
            Ok(Some(line)) => {
                if line.is_empty() {
                    continue;
                }
                match parse_event(&line) {
                    Some(ev) => {
                        // 关键事件：alive 仍在 + tx.send 成功
                        if tx.send(ev).await.is_err() {
                            tracing::debug!(
                                "[claude:{}] stdout reader: receiver dropped, exit",
                                session_id
                            );
                            break;
                        }
                    }
                    None => {
                        // 不可解析或忽略的类型；继续读下一行
                        tracing::debug!("[claude:{}] ignored line: {}", session_id, line);
                    }
                }
            }
            Ok(None) => {
                // stdout EOF（子进程结束或 pipe 关闭）
                tracing::info!("[claude:{}] stdout EOF", session_id);
                let _ = tx
                    .send(Event::Closed)
                    .await;
                alive.store(false, Ordering::Release);
                break;
            }
            Err(e) => {
                tracing::error!("[claude:{}] stdout read error: {}", session_id, e);
                let _ = tx.send(Event::Error(format!("stdout read error: {e}"))).await;
                break;
            }
        }
    }
}

/// 单行 NDJSON → Event enum。
///
/// 解析失败返回 None（调用方 log + 继续读下一行）。
fn parse_event(line: &str) -> Option<Event> {
    let v: Value = serde_json::from_str(line).ok()?;
    let ty = v.get("type")?.as_str()?;

    match ty {
        // system init / user replay → 忽略
        "system" | "user" => None,

        // assistant message：text 或 tool_use
        "assistant" => {
            let message = v.get("message")?;
            let content = message.get("content")?.as_array()?;
            // v1：仅取第一个 text block；多 block 的 content 暂不支持。
            for block in content {
                let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match block_type {
                    "text" => {
                        let text = block.get("text")?.as_str()?.to_string();
                        return Some(Event::Text(text));
                    }
                    "tool_use" => {
                        let name = block
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let args = block.get("input").cloned().unwrap_or(Value::Null);
                        return Some(Event::ToolUse { name, args });
                    }
                    _ => continue,
                }
            }
            None
        }

        // result（turn 结束）
        "result" => {
            let duration_ms = v
                .get("duration_ms")
                .and_then(|d| d.as_u64())
                .unwrap_or(0);
            let usage = parse_usage(v.get("usage"));
            let subtype = v.get("subtype").and_then(|s| s.as_str()).unwrap_or("");
            if subtype == "success" {
                Some(Event::Result { usage, duration_ms })
            } else {
                // error_max_turns / error_tool_result_missing / etc.
                Some(Event::Error(format!("claude result: {subtype}")))
            }
        }

        // 权限请求（dispatcher 必须回 respond_permission）
        "control_request" => {
            let request_id = v
                .get("request_id")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .to_string();
            let request = v.get("request").cloned().unwrap_or(Value::Null);
            // tool 字段：subtype=can_use_tool 时是 tool 名；其它 subtype 取
            // request.tool_name 兼容老格式。
            let tool = request
                .get("tool_name")
                .or_else(|| request.get("subtype"))
                .and_then(|t| t.as_str())
                .unwrap_or("unknown")
                .to_string();
            let args = request.get("input").cloned().unwrap_or(Value::Null);
            Some(Event::PermissionRequest {
                request_id,
                tool,
                args,
            })
        }

        _ => None,
    }
}

/// 解析 Claude Code 的 usage 字段。
///
/// 字段名：input_tokens / output_tokens / cache_read_input_tokens。
fn parse_usage(v: Option<&Value>) -> Usage {
    let mut usage = Usage::default();
    if let Some(v) = v {
        if let Some(n) = v.get("input_tokens").and_then(|x| x.as_u64()) {
            usage.input_tokens = n;
        }
        if let Some(n) = v.get("output_tokens").and_then(|x| x.as_u64()) {
            usage.output_tokens = n;
        }
        if let Some(n) = v.get("cache_read_input_tokens").and_then(|x| x.as_u64()) {
            usage.cache_read_tokens = n;
        }
    }
    usage
}

/// ClaudeCodeSession：单次会话的 stdio 句柄 + 子进程 alive 状态。
///
/// 字段使用两套 mutex：
/// - `stdin` 用 `tokio::sync::Mutex`（写 stdin 需要跨 `.await` 持锁）
/// - `events_rx` 用 `std::sync::Mutex`（take_events 是同步调用，
///   不能用 tokio::Mutex::blocking_lock()，否则在 tokio runtime 里 panic）
pub struct ClaudeCodeSession {
    session_id: String,
    /// `None` 表示已 close（take 走了）。
    stdin: Mutex<Option<tokio::process::ChildStdin>>,
    /// false 表示子进程已退出；reader task 不再发新事件。
    alive: Arc<AtomicBool>,
    /// 一次性 ownership 转移的 mpsc receiver。
    events_rx: StdMutex<Option<mpsc::Receiver<Event>>>,
}

impl ClaudeCodeSession {
    /// 写一条 JSON line 到 stdin（NDJSON 协议：每条消息一行 JSON + '\n'）。
    async fn write_line(&self, value: Value) -> Result<()> {
        let mut guard = self.stdin.lock().await;
        let stdin = guard
            .as_mut()
            .ok_or_else(|| Error::agent("session already closed".to_string()))?;
        let mut line = serde_json::to_vec(&value)
            .map_err(|e| Error::agent(format!("serialize stdin: {e}")))?;
        line.push(b'\n');
        stdin
            .write_all(&line)
            .await
            .map_err(|e| Error::agent(format!("write stdin: {e}")))?;
        stdin
            .flush()
            .await
            .map_err(|e| Error::agent(format!("flush stdin: {e}")))?;
        Ok(())
    }
}

#[async_trait]
impl AgentSession for ClaudeCodeSession {
    async fn send(&self, prompt: &str, _attachments: &[Attachment]) -> Result<()> {
        // v1 仅支持纯文本 prompt。attachments 留 v2（Claude Code 的 image 类型支持）。
        let value = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [{"type": "text", "text": prompt}]
            }
        });
        self.write_line(value).await
    }

    async fn respond_permission(
        &self,
        request_id: &str,
        result: PermissionResult,
    ) -> Result<()> {
        // v1：所有 permission request 自动按传入 result 回执（默认 Allow）。
        // v2（permission hook）：dispatcher 转给用户/规则引擎后再回执。
        let behavior = match result {
            PermissionResult::Allow | PermissionResult::AllowAlways => "allow",
            PermissionResult::Deny => "deny",
        };
        let value = serde_json::json!({
            "type": "control_response",
            "response": {
                "request_id": request_id,
                "behavior": behavior,
            }
        });
        self.write_line(value).await
    }

    fn take_events(&mut self) -> mpsc::Receiver<Event> {
        // 一次性 ownership 转移。多次调用 panic（防误用）。
        self.events_rx
            .lock()
            .expect("events_rx mutex poisoned")
            .take()
            .expect("ClaudeCodeSession::take_events called twice")
    }

    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }

    async fn close(&self) -> Result<()> {
        // v1 三段式关闭：
        // 1. close stdin（让 claude 收到 EOF，自然退出）
        if let Some(mut stdin) = self.stdin.lock().await.take() {
            let _ = stdin.shutdown().await;
        }
        // 2/3. SIGTERM group kill / SIGKILL：v1 简化用 tokio Child::kill
        // （不带 process group）。v2 加 nix crate 做 group kill。
        // 注：child handle 已在 start_session 时移交给 wait task，
        // 这边只能等待 alive 标志。alive 由 wait task 在 child 退出时
        // 置 false；close 是 best-effort，不主动 await 子进程退出。
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// parse_event：assistant text 消息 → Event::Text。
    #[test]
    fn test_parse_event_assistant_text() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"hello"}]}}"#;
        match parse_event(line) {
            Some(Event::Text(s)) => assert_eq!(s, "hello"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    /// parse_event：result success → Event::Result 含 usage。
    #[test]
    fn test_parse_event_result_success() {
        let line = r#"{"type":"result","subtype":"success","duration_ms":1234,"usage":{"input_tokens":10,"output_tokens":20,"cache_read_input_tokens":5}}"#;
        match parse_event(line) {
            Some(Event::Result { usage, duration_ms }) => {
                assert_eq!(duration_ms, 1234);
                assert_eq!(usage.input_tokens, 10);
                assert_eq!(usage.output_tokens, 20);
                assert_eq!(usage.cache_read_tokens, 5);
            }
            other => panic!("expected Result, got {other:?}"),
        }
    }

    /// parse_event：result error_max_turns → Event::Error。
    #[test]
    fn test_parse_event_result_error() {
        let line = r#"{"type":"result","subtype":"error_max_turns","duration_ms":100}"#;
        match parse_event(line) {
            Some(Event::Error(e)) => assert!(e.contains("error_max_turns")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    /// parse_event：control_request → Event::PermissionRequest。
    #[test]
    fn test_parse_event_control_request() {
        let line = r#"{"type":"control_request","request_id":"req-1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"cmd":"ls"}}}"#;
        match parse_event(line) {
            Some(Event::PermissionRequest { request_id, tool, .. }) => {
                assert_eq!(request_id, "req-1");
                assert_eq!(tool, "Bash");
            }
            other => panic!("expected PermissionRequest, got {other:?}"),
        }
    }

    /// parse_event：system / user → None（忽略）。
    #[test]
    fn test_parse_event_ignores_system_and_user() {
        assert!(parse_event(r#"{"type":"system","subtype":"init"}"#).is_none());
        assert!(parse_event(
            r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"hi"}]}}"#
        )
        .is_none());
    }

    /// parse_event：assistant tool_use → Event::ToolUse。
    #[test]
    fn test_parse_event_tool_use() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"path":"/tmp/x"}}]}}"#;
        match parse_event(line) {
            Some(Event::ToolUse { name, .. }) => assert_eq!(name, "Edit"),
            other => panic!("expected ToolUse, got {other:?}"),
        }
    }

    /// parse_event：无效 JSON → None（不 panic，让 reader 继续）。
    #[test]
    fn test_parse_event_invalid_json_returns_none() {
        assert!(parse_event("not json").is_none());
        assert!(parse_event("").is_none());
    }

    /// ClaudeCodeAgent::new 默认路径是 "claude"，args 含 stream-json。
    #[test]
    fn test_claude_code_agent_default() {
        let agent = ClaudeCodeAgent::new();
        assert_eq!(agent.claude_path, PathBuf::from("claude"));
        assert!(agent
            .default_args
            .iter()
            .any(|a| a == "stream-json"));
        assert!(agent
            .default_args
            .windows(2)
            .any(|w| w[0] == "--output-format" && w[1] == "stream-json"));
        assert!(agent
            .default_args
            .windows(2)
            .any(|w| w[0] == "--input-format" && w[1] == "stream-json"));
        assert!(agent
            .default_args
            .windows(2)
            .any(|w| w[0] == "--permission-prompt-tool" && w[1] == "stdio"));
    }

    /// ClaudeCodeAgent::with_path 替换 binary 路径。
    #[test]
    fn test_claude_code_agent_with_path() {
        let p = PathBuf::from("/tmp/mock-claude.sh");
        let agent = ClaudeCodeAgent::with_path(p.clone());
        assert_eq!(agent.claude_path, p);
    }

    /// ClaudeCodeAgent::name 必须返回 "claude-code"。
    #[test]
    fn test_agent_name() {
        let agent = ClaudeCodeAgent::new();
        assert_eq!(agent.name(), "claude-code");
    }

    /// `parse_usage` 各字段映射正确（含 cache_read → cache_read_tokens）。
    #[test]
    fn test_parse_usage() {
        let v: Value = serde_json::json!({
            "input_tokens": 100,
            "output_tokens": 200,
            "cache_read_input_tokens": 30,
        });
        let u = parse_usage(Some(&v));
        assert_eq!(u.input_tokens, 100);
        assert_eq!(u.output_tokens, 200);
        assert_eq!(u.cache_read_tokens, 30);
    }
}

/// 真实 mock：spawn `/bin/sh` 跑一个脚本，读 stdin NDJSON，按预定义顺序
/// 回写 stdout events。验证 `start_session → send → events` 整条流水线。
#[cfg(test)]
mod mock_binary_tests {
    use super::*;
    use std::path::Path;
    use std::time::Duration;
    use tokio::time::timeout;

    /// 写一个临时 shell 脚本到 tmpdir，路径返回。
    /// 脚本行为：
    /// - 读 stdin 一行 JSON → 写 stdout {"type":"assistant", ... "text":"echo: <text>"}。
    /// - 收到含 "shutdown" 的行 → 写 result event + 退出。
    fn write_mock_script(dir: &Path) -> PathBuf {
        let script = dir.join("mock-claude.sh");
        std::fs::write(
            &script,
            r#"#!/bin/sh
# mock claude binary for ntd-connect tests
# Reads NDJSON from stdin, echoes back as assistant messages.
while IFS= read -r line; do
    if echo "$line" | grep -q "shutdown"; then
        echo '{"type":"result","subtype":"success","duration_ms":42,"usage":{"input_tokens":7,"output_tokens":3}}'
        echo '{"type":"user","message":{"role":"user","content":[{"type":"text","text":"echo back"}]}}'
        exit 0
    fi
    # extract text field via simple grep + sed
    text=$(echo "$line" | sed -n 's/.*"text":"\([^"]*\)".*/\1/p')
    echo "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"echo: $text\"}]}}"
done
"#,
        )
        .expect("write mock script");
        // chmod +x
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).unwrap();
        script
    }

    /// 端到端：start_session → send("hi") → events 收到 "echo: hi" → 第二次 send("shutdown") → result event。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mock_binary_full_roundtrip() {
        // tmpdir for mock script
        let tmpdir = std::env::temp_dir().join(format!(
            "ntd-connect-claude-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&tmpdir).unwrap();
        let mock_path = write_mock_script(&tmpdir);

        let agent = ClaudeCodeAgent::with_path(mock_path);
        let mut session = agent
            .start_session(&AgentContext::default(), None)
            .await
            .expect("start_session");
        assert!(session.alive());
        assert!(!session.session_id().is_empty());

        let mut events_rx = session.take_events();

        // 第一轮 send("hi") → 期望 events: Text("echo: hi")。
        session.send("hi", &[]).await.expect("send");
        let ev = timeout(Duration::from_secs(5), events_rx.recv())
            .await
            .expect("recv timeout")
            .expect("recv Some");
        match ev {
            Event::Text(s) => assert_eq!(s, "echo: hi"),
            other => panic!("expected Text, got {other:?}"),
        }

        // 第二轮 send("shutdown now") → 期望 events: Result(usage, duration_ms=42) + Closed。
        session
            .send("shutdown now", &[])
            .await
            .expect("send shutdown");
        // 接下来：user replay event（mock 脚本发送），result event，channel close。
        // 由于 mock 脚本发了 user + result 然后退出。
        let mut got_result = false;
        let mut got_closed = false;
        for _ in 0..5 {
            match timeout(Duration::from_secs(5), events_rx.recv())
                .await
                .expect("recv timeout")
            {
                Some(Event::Result { duration_ms, .. }) => {
                    assert_eq!(duration_ms, 42);
                    got_result = true;
                }
                Some(Event::Closed) => {
                    got_closed = true;
                    break;
                }
                Some(_) => continue, // user replay 等忽略
                None => break,
            }
        }
        assert!(got_result, "应收到 Result event");
        assert!(got_closed, "应收到 Closed event");
        assert!(!session.alive(), "alive 在子进程退出后应 false");

        // cleanup tmpdir
        let _ = std::fs::remove_dir_all(&tmpdir);
    }

    /// respond_permission 写一条 control_response JSON 到 stdin。
    /// mock 脚本当前不读 control_response，但语法上应不阻塞。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_respond_permission_writes_to_stdin() {
        let tmpdir = std::env::temp_dir().join(format!(
            "ntd-connect-claude-perm-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&tmpdir).unwrap();
        let mock_path = write_mock_script(&tmpdir);

        let agent = ClaudeCodeAgent::with_path(mock_path);
        let mut session = agent
            .start_session(&AgentContext::default(), None)
            .await
            .expect("start_session");
        let _events_rx = session.take_events();

        // respond_permission 不应 panic；写一条 stdin 行让 mock 收到。
        session
            .respond_permission("req-test", PermissionResult::Allow)
            .await
            .expect("respond_permission");

        // 让 mock 处理一下
        tokio::time::sleep(Duration::from_millis(50)).await;

        // close stdin 让 mock 走完
        session.close().await.expect("close");

        let _ = std::fs::remove_dir_all(&tmpdir);
    }
}
