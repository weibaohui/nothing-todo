# CLAUDE.md

## 项目概述
aitodo 是一个 AI Todo 应用，基于 Rust 后端 + React 前端，支持 Claude Code 和 JoinAI 执行器。

## 开发流程

**每次完成功能开发后，执行 `make restart` 重启服务以便调试。**

## 技术栈
- 后端: Rust (Axum框架)
- 前端: React + Vite + Ant Design
- 数据库: SQLite

## 常用命令

```bash
make install    # 构建并安装
make start     # 启动服务 (需要先 install)
make stop      # 停止服务
make restart   # 重启服务 (开发调试时常用)
make dev       # 开发模式 (前后端分离)
make build     # 仅构建
make clean     # 清理构建产物
```

## 端口
- 前端: 5173 (开发模式)
- 后端: 8088

## 目录结构
- `backend/` - Rust 后端代码
- `frontend/` - React 前端代码
- `tunnel.sh` - 内网穿透脚本
