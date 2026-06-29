# Mobilecoder 启动命令

## 命令格式

```bash
mobile run --agent yolo --format json <消息内容>
```

## 参数说明

| 参数 | 说明 |
|------|------|
| `run` | 运行子命令 |
| `--agent yolo` | 使用 yolo 模式（自动批准） |
| `--format json` | 输出 JSONL 格式 |

## 会话恢复

```bash
mobile run --agent yolo --format json -s <session_id> <消息内容>
```

## 可执行文件路径

- 配置路径: `~/.mobile-coder/mobile`
- 默认二进制名: `mobile`
