# Opencode 启动命令

## 命令格式

```bash
opencode -p "<消息内容>" -f json
```

## 参数说明

| 参数 | 说明 |
|------|------|
| `-p, --prompt` | 以非交互模式运行 |
| `-f, --output-format json` | 输出 JSON 格式 |
| `-q, --quiet` | 隐藏 spinner |

## 历史版本

旧版本使用:
```bash
opencode run --format json --dangerously-skip-permissions <消息内容>
```
