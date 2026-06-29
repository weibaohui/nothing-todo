# Opencode 启动命令

## 命令格式

有两个版本的 opencode：

### 新版（Go v0.0.55）
路径: `~/.local/bin/opencode`

```bash
opencode -p "<消息内容>" -f json
```

输出简化版 `{"response": "..."}`。

### 旧版（npm，推荐，含丰富 JSONL 事件流）
路径: `/usr/local/lib/node_modules/opencode-ai/bin/opencode.exe`

```bash
opencode run --format json --dangerously-skip-permissions <消息内容>
```

输出 JSONL 流（step_start / tool_use / text / step_finish 等事件）。

## 参数说明（旧版）

| 参数 | 说明 |
|------|------|
| `run` | 运行子命令 |
| `--format json` | 输出 JSONL 格式 |
| `--dangerously-skip-permissions` | 跳过交互式权限确认 |

## 会话恢复

```bash
opencode run --format json -s <session_id> <消息内容>
```
