#!/bin/bash

# SnapRAG GPU编译脚本 - 适用于Debian/Ubuntu服务器
# 解决CUDA头文件冲突问题

set -e

echo "🚀 SnapRAG GPU编译脚本"
echo "================================"

# 检查CUDA环境
if ! command -v nvcc &> /dev/null; then
    echo "❌ 错误: 未找到CUDA编译器 (nvcc)"
    echo "请安装CUDA Toolkit: https://developer.nvidia.com/cuda-downloads"
    exit 1
fi

# 检查GCC
if ! command -v gcc &> /dev/null; then
    echo "❌ 错误: 未找到GCC编译器"
    echo "请安装: sudo apt-get install build-essential"
    exit 1
fi

echo "✅ CUDA环境检查通过"
echo "CUDA版本: $(nvcc --version | grep release)"
echo "GCC版本: $(gcc --version | head -n1)"

# 设置环境变量解决CUDA编译问题
export NVCC_CCBIN=/usr/bin/gcc
export CUDA_COMPUTE_CAP=61  # 根据你的GPU调整

echo ""
echo "🔧 设置编译环境变量:"
echo "NVCC_CCBIN=$NVCC_CCBIN"
echo "CUDA_COMPUTE_CAP=$CUDA_COMPUTE_CAP"

echo ""
echo "📦 开始编译SnapRAG (GPU版本)..."
echo "================================"

# 编译命令
if cargo build --release --features local-gpu; then
    echo ""
    echo "✅ 编译成功！"
    echo "================================"
    echo "🎉 SnapRAG GPU版本已编译完成"
    echo "📁 可执行文件位置: target/release/snaprag"
    echo ""
    echo "🚀 使用方法:"
    echo "1. 配置config.toml使用local_gpu provider"
    echo "2. 运行: ./target/release/snaprag"
    echo ""
else
    echo ""
    echo "❌ 编译失败！"
    echo "================================"
    echo "🔍 可能的解决方案:"
    echo "1. 检查CUDA版本兼容性"
    echo "2. 更新GCC版本"
    echo "3. 尝试不同的CUDA_COMPUTE_CAP值"
    echo "4. 使用CPU版本: cargo build --release"
    echo ""
    exit 1
fi
