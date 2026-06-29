# Codebuddy 启动命令

## 命令格式

```bash
codebuddy -p --output-format stream-json --verbose <消息内容>
```

## 参数说明

| 参数 | 说明 |
|------|------|
| `-p` | 以 prompt 模式运行（非交互） |
| `--output-format stream-json` | 输出 JSONL 流式格式（Claude Protocol） |
| `--verbose` | 详细输出 |

## 输出格式

Codebuddy 使用 Claude Protocol 格式，与 ClaudeCode 完全一致。
