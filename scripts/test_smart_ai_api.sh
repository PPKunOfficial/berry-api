#!/bin/bash

# SmartAI API 测试脚本
# 用于测试SmartAI权重查看API的功能

BASE_URL="http://localhost:3000"

echo "🚀 SmartAI API 测试脚本"
echo "========================"

# 检查服务是否运行
echo "📡 检查服务状态..."
if ! curl -s "$BASE_URL/health" > /dev/null; then
    echo "❌ 服务未运行，请先启动Berry API服务"
    exit 1
fi
echo "✅ 服务正常运行"
echo

# 测试1: 获取所有SmartAI模型的基本权重信息
echo "📊 测试1: 获取所有SmartAI模型的基本权重信息"
echo "GET $BASE_URL/smart-ai/weights"
echo "----------------------------------------"
curl -s "$BASE_URL/smart-ai/weights" | jq '.' || echo "❌ 请求失败或响应不是有效的JSON"
echo
echo

# 测试2: 获取详细信息，包含所有后端
echo "📊 测试2: 获取详细信息（包含所有后端）"
echo "GET $BASE_URL/smart-ai/weights?detailed=true&enabled_only=false"
echo "----------------------------------------"
curl -s "$BASE_URL/smart-ai/weights?detailed=true&enabled_only=false" | jq '.' || echo "❌ 请求失败或响应不是有效的JSON"
echo
echo

# 测试3: 获取模型列表，找到一个SmartAI模型进行测试
echo "📊 测试3: 查找SmartAI模型"
echo "----------------------------------------"
RESPONSE=$(curl -s "$BASE_URL/smart-ai/weights" 2>/dev/null)
SMART_AI_MODELS=$(echo "$RESPONSE" | jq -r '.available_models[]? | "\(.key) (\(.name))"' 2>/dev/null)

if [ -z "$SMART_AI_MODELS" ]; then
    echo "⚠️  没有找到使用SmartAI策略的模型"
    echo "请确保配置文件中至少有一个模型使用 strategy = \"smart_ai\""
else
    echo "✅ 找到以下SmartAI模型:"
    echo "$SMART_AI_MODELS"
    echo

    # 测试4: 获取第一个模型的详细信息
    FIRST_MODEL_KEY=$(echo "$RESPONSE" | jq -r '.available_models[0].key' 2>/dev/null)
    FIRST_MODEL_NAME=$(echo "$RESPONSE" | jq -r '.available_models[0].name' 2>/dev/null)

    if [ "$FIRST_MODEL_KEY" != "null" ] && [ -n "$FIRST_MODEL_KEY" ]; then
        echo "📊 测试4a: 使用模型键名 '$FIRST_MODEL_KEY' 获取详细权重信息"
        echo "GET $BASE_URL/smart-ai/models/$FIRST_MODEL_KEY/weights?detailed=true"
        echo "----------------------------------------"
        curl -s "$BASE_URL/smart-ai/models/$FIRST_MODEL_KEY/weights?detailed=true" | jq '.' || echo "❌ 请求失败或响应不是有效的JSON"
        echo
        echo

        echo "📊 测试4b: 使用显示名称 '$FIRST_MODEL_NAME' 获取详细权重信息"
        echo "GET $BASE_URL/smart-ai/models/$FIRST_MODEL_NAME/weights?detailed=true"
        echo "----------------------------------------"
        curl -s "$BASE_URL/smart-ai/models/$FIRST_MODEL_NAME/weights?detailed=true" | jq '.' || echo "❌ 请求失败或响应不是有效的JSON"
        echo
        echo
    fi
fi

# 测试5: 测试不存在的模型
echo "📊 测试5: 测试不存在的模型（错误处理）"
echo "GET $BASE_URL/smart-ai/models/non_existent_model/weights"
echo "----------------------------------------"
curl -s "$BASE_URL/smart-ai/models/non_existent_model/weights" | jq '.' || echo "❌ 请求失败或响应不是有效的JSON"
echo
echo

# 测试6: 权重分布分析
echo "📊 测试6: 权重分布分析"
echo "----------------------------------------"
echo "各模型的权重分布:"
curl -s "$BASE_URL/smart-ai/weights" | jq -r '
.models[] | 
"模型: \(.name)" as $model |
.stats.weight_distribution | 
to_entries[] | 
"\($model) - \(.key): \(.value)"
' 2>/dev/null || echo "❌ 分析失败"
echo

# 测试7: Premium后端检查
echo "📊 测试7: Premium后端检查"
echo "----------------------------------------"
echo "Premium后端列表:"
curl -s "$BASE_URL/smart-ai/weights?detailed=true" | jq -r '
.models[] | 
.backends[] | 
select(.is_premium == true) | 
"模型: \(.model) - 提供商: \(.provider) - 有效权重: \(.effective_weight) - 信心度: \(.confidence)"
' 2>/dev/null || echo "❌ 检查失败"
echo

# 测试8: 健康状态概览
echo "📊 测试8: 健康状态概览"
echo "----------------------------------------"
echo "各模型健康状态统计:"
curl -s "$BASE_URL/smart-ai/weights" | jq -r '
.models[] | 
"模型: \(.name) - 总后端: \(.stats.total_backends) - 健康后端: \(.stats.healthy_backends) - Premium后端: \(.stats.premium_backends) - 平均信心度: \(.stats.average_confidence)"
' 2>/dev/null || echo "❌ 统计失败"
echo

echo "🎉 测试完成！"
echo
echo "💡 使用建议:"
echo "1. 定期运行此脚本监控SmartAI状态"
echo "2. 关注Premium后端的权重分布，确保成本控制"
echo "3. 监控平均信心度，识别不稳定的后端"
echo "4. 使用详细模式查看具体的健康状态信息"
echo
echo "📚 更多信息请查看: docs/smart_ai_api.md"
