# Atomcode 启动命令

## 命令格式

```bash
atomcode -v --dangerously-skip-permissions -p <消息内容>
```

## 参数说明

| 参数 | 说明 |
|------|------|
| `-v` | 详细输出模式 |
| `--dangerously-skip-permissions` | 跳过交互式权限确认（自动化模式） |
| `-p` | 以 prompt 模式运行（非交互） |

## 输出特点

- **stdout**：纯文本，AI 的回复内容直接输出到 stdout
- **stderr**：结构化事件，以 `[xxx]` 前缀标记
