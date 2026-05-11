#!/bin/bash

# 使用 hostc 将本地 8088 端口开放出去，返回 public URL

PORT=${1:-8088}
mkdir -p ~/.ntd

# 如果存在旧的 tunnel pid，先杀掉
PID_FILE=~/.ntd/tunnel.pid
if [ -f "$PID_FILE" ]; then
    OLD_PID=$(cat "$PID_FILE")
    if [ -n "$OLD_PID" ] && kill -0 "$OLD_PID" 2>/dev/null; then
        echo "Stopping old tunnel (PID: $OLD_PID)"
        kill "$OLD_PID" 2>/dev/null
        # 等待退出，最多 3s
        for _ in 1 2 3; do
            kill -0 "$OLD_PID" 2>/dev/null || break
            sleep 1
        done
        kill -9 "$OLD_PID" 2>/dev/null
    fi
fi

# 顺手清理任何残留的 hostc 8088 进程（防止脚本异常退出留下的孤儿）
pkill -f "hostc ${PORT}" 2>/dev/null

# 启动 hostc 隧道
hostc ${PORT} > /tmp/hostc_output.txt 2>&1 &
HOSTC_PID=$!

# 保存 PID
echo $HOSTC_PID > "$PID_FILE"

# 轮询等待 Public URL（最多 15s）
PUBLIC_URL=""
for _ in $(seq 1 30); do
    if [ -f /tmp/hostc_output.txt ]; then
        PUBLIC_URL=$(grep "Public URL:" /tmp/hostc_output.txt | sed 's/.*Public URL: //')
        if [ -n "$PUBLIC_URL" ]; then
            break
        fi
    fi
    sleep 0.5
done

# 显示 hostc 输出
if [ -f /tmp/hostc_output.txt ]; then
    cat /tmp/hostc_output.txt
fi

if [ -z "$PUBLIC_URL" ]; then
    echo "Error: failed to capture Public URL within 15s" >&2
    exit 1
fi

echo $PUBLIC_URL > ~/.ntd/tunnel.url

echo ""
echo "Tunnel PID: $HOSTC_PID"
echo "Public URL saved to ~/.ntd/tunnel.url"
