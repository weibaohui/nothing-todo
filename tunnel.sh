#!/bin/bash

# 使用 hostc 将本地 8088 端口开放出去，返回 public URL

mkdir -p ~/.aitodo

# 启动 hostc 隧道
hostc 8088 > /tmp/hostc_output.txt 2>&1 &
HOSTC_PID=$!

# 保存 PID
echo $HOSTC_PID > ~/.aitodo/tunnel.pid

# 等待 URL 输出
sleep 2

# 读取并显示结果
if [ -f /tmp/hostc_output.txt ]; then
    cat /tmp/hostc_output.txt
fi

# 提取 Public URL 并保存
PUBLIC_URL=$(grep "Public URL:" /tmp/hostc_output.txt | sed 's/.*Public URL: //')
echo $PUBLIC_URL > ~/.aitodo/tunnel.url

echo ""
echo "Tunnel PID: $HOSTC_PID"
echo "Public URL saved to ~/.aitodo/tunnel.url"
