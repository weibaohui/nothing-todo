# Codewhale 启动命令

## 命令格式

```bash
codewhale exec --auto --output-format stream-json <消息内容>
```

## 参数说明

| 参数 | 说明 |
|------|------|
| `exec` | 执行子命令 |
| `--auto` | 自动批准工具调用 |
| `--output-format stream-json` | 输出 NDJSON 流式格式 |

## 会话恢复

```bash
codewhale exec --auto --output-format stream-json --session-id <session_id> <消息内容>
```
