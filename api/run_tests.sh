#!/bin/bash

# Berry API 测试运行脚本
# 用于运行各种类型的测试和生成报告

set -e

echo "🧪 Berry API 测试套件"
echo "===================="

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# 函数：打印带颜色的消息
print_status() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# 检查依赖
check_dependencies() {
    echo "🔍 检查依赖..."
    
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo 未安装"
        exit 1
    fi
    
    if ! command -v cargo-tarpaulin &> /dev/null; then
        print_warning "cargo-tarpaulin 未安装，正在安装..."
        cargo install cargo-tarpaulin
    fi
    
    print_status "依赖检查完成"
}

# 运行单元测试
run_unit_tests() {
    echo ""
    echo "🧪 运行单元测试..."
    
    if cargo test --lib; then
        print_status "单元测试通过"
    else
        print_error "单元测试失败"
        exit 1
    fi
}

# 运行集成测试
run_integration_tests() {
    echo ""
    echo "🔗 运行集成测试..."
    
    if cargo test --test '*'; then
        print_status "集成测试通过"
    else
        print_warning "集成测试失败或不存在"
    fi
}

# 运行所有测试
run_all_tests() {
    echo ""
    echo "🚀 运行所有测试..."
    
    if cargo test; then
        print_status "所有测试通过"
    else
        print_error "部分测试失败"
        exit 1
    fi
}

# 生成覆盖率报告
generate_coverage() {
    echo ""
    echo "📊 生成覆盖率报告..."
    
    if cargo tarpaulin --out Html --output-dir coverage; then
        print_status "覆盖率报告生成完成"
        echo "📁 报告位置: coverage/tarpaulin-report.html"
    else
        print_error "覆盖率报告生成失败"
    fi
}

# 运行性能测试
run_bench_tests() {
    echo ""
    echo "⚡ 运行性能测试..."
    
    if cargo test --release --benches; then
        print_status "性能测试完成"
    else
        print_warning "性能测试失败或不存在"
    fi
}

# 代码格式检查
check_format() {
    echo ""
    echo "🎨 检查代码格式..."
    
    if cargo fmt -- --check; then
        print_status "代码格式正确"
    else
        print_warning "代码格式需要调整"
        echo "运行 'cargo fmt' 来修复格式问题"
    fi
}

# 代码质量检查
check_clippy() {
    echo ""
    echo "🔍 运行 Clippy 检查..."
    
    if cargo clippy -- -D warnings; then
        print_status "Clippy 检查通过"
    else
        print_warning "Clippy 发现了一些问题"
    fi
}

# 显示帮助信息
show_help() {
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  unit        运行单元测试"
    echo "  integration 运行集成测试"
    echo "  all         运行所有测试 (默认)"
    echo "  coverage    生成覆盖率报告"
    echo "  bench       运行性能测试"
    echo "  format      检查代码格式"
    echo "  clippy      运行 Clippy 检查"
    echo "  full        运行完整测试套件 (测试+覆盖率+格式+Clippy)"
    echo "  help        显示此帮助信息"
}

# 运行完整测试套件
run_full_suite() {
    echo "🎯 运行完整测试套件..."
    check_dependencies
    check_format
    check_clippy
    run_all_tests
    generate_coverage
    
    echo ""
    echo "🎉 完整测试套件执行完成！"
}

# 主逻辑
case "${1:-all}" in
    "unit")
        check_dependencies
        run_unit_tests
        ;;
    "integration")
        check_dependencies
        run_integration_tests
        ;;
    "all")
        check_dependencies
        run_all_tests
        ;;
    "coverage")
        check_dependencies
        generate_coverage
        ;;
    "bench")
        check_dependencies
        run_bench_tests
        ;;
    "format")
        check_format
        ;;
    "clippy")
        check_clippy
        ;;
    "full")
        run_full_suite
        ;;
    "help"|"-h"|"--help")
        show_help
        ;;
    *)
        echo "未知选项: $1"
        show_help
        exit 1
        ;;
esac

echo ""
echo "✨ 测试完成！"
