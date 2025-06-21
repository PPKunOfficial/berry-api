#!/bin/bash

# =================================================================
# 测试 Release 构建脚本
# 用于本地测试 GitHub Release 构建流程
# =================================================================

set -e  # 遇到错误立即退出

echo "🚀 开始测试 Release 构建流程..."

# 检查是否安装了必要工具
echo "📋 检查环境..."
if ! command -v cargo &> /dev/null; then
    echo "❌ 错误: 未找到 cargo，请安装 Rust 工具链"
    exit 1
fi

# 模拟版本号（如果没有提供）
VERSION=${1:-"v0.1.0-test"}
echo "📦 使用版本号: $VERSION"

# 清理之前的构建产物
echo "🧹 清理之前的构建产物..."
rm -rf ./release-assets
rm -rf ./target/x86_64-unknown-linux-gnu

# 编译 Rust 二进制文件
echo "🔨 编译 Rust 二进制文件..."
echo "   - 目标: x86_64-unknown-linux-gnu"
echo "   - 功能: observability"
echo "   - 模式: release"

cargo build --workspace --release --features observability --target x86_64-unknown-linux-gnu

# 验证编译结果
echo "✅ 验证编译结果..."
if [ ! -f "target/x86_64-unknown-linux-gnu/release/berry-api" ]; then
    echo "❌ 错误: berry-api 二进制文件未找到"
    exit 1
fi

if [ ! -f "target/x86_64-unknown-linux-gnu/release/berry-cli" ]; then
    echo "❌ 错误: berry-cli 二进制文件未找到"
    exit 1
fi

echo "   ✓ berry-api: $(file target/x86_64-unknown-linux-gnu/release/berry-api)"
echo "   ✓ berry-cli: $(file target/x86_64-unknown-linux-gnu/release/berry-cli)"

# 准备 Release 文件
echo "📦 准备 Release 文件..."
mkdir -p ./release-assets

# 复制二进制文件并重命名
cp target/x86_64-unknown-linux-gnu/release/berry-api ./release-assets/berry-api-linux-x86_64
cp target/x86_64-unknown-linux-gnu/release/berry-cli ./release-assets/berry-cli-linux-x86_64

# 创建压缩包
cd release-assets
tar -czf berry-api-${VERSION}-linux-x86_64.tar.gz berry-api-linux-x86_64 berry-cli-linux-x86_64

# 计算校验和
sha256sum berry-api-${VERSION}-linux-x86_64.tar.gz > berry-api-${VERSION}-linux-x86_64.tar.gz.sha256

# 显示文件信息
echo "📊 Release 文件信息:"
ls -la
echo ""
echo "🔐 SHA256 校验和:"
cat berry-api-${VERSION}-linux-x86_64.tar.gz.sha256

cd ..

echo ""
echo "✅ Release 构建完成!"
echo ""
echo "📁 生成的文件:"
echo "   - release-assets/berry-api-${VERSION}-linux-x86_64.tar.gz"
echo "   - release-assets/berry-api-${VERSION}-linux-x86_64.tar.gz.sha256"
echo ""
echo "🧪 测试解压:"
echo "   cd /tmp"
echo "   tar -xzf $(pwd)/release-assets/berry-api-${VERSION}-linux-x86_64.tar.gz"
echo "   ./berry-api-linux-x86_64 --version"
echo ""
