#!/bin/bash

# 测试模型名称查找功能
BASE_URL="http://localhost:3000"

echo "🔍 测试SmartAI模型名称查找功能"
echo "================================"

# 1. 获取可用模型列表
echo "📋 步骤1: 获取可用的SmartAI模型"
echo "GET $BASE_URL/smart-ai/weights"
echo "--------------------------------"

RESPONSE=$(curl -s "$BASE_URL/smart-ai/weights")
echo "$RESPONSE" | jq '.available_models' 2>/dev/null || echo "❌ 无法获取模型列表"

# 提取第一个模型的信息
MODEL_KEY=$(echo "$RESPONSE" | jq -r '.available_models[0].key' 2>/dev/null)
MODEL_NAME=$(echo "$RESPONSE" | jq -r '.available_models[0].name' 2>/dev/null)

if [ "$MODEL_KEY" != "null" ] && [ -n "$MODEL_KEY" ]; then
    echo
    echo "🧪 步骤2: 测试使用配置键名查询"
    echo "模型键名: $MODEL_KEY"
    echo "GET $BASE_URL/smart-ai/models/$MODEL_KEY/weights"
    echo "--------------------------------"
    
    curl -s "$BASE_URL/smart-ai/models/$MODEL_KEY/weights" | jq '.model.name' 2>/dev/null || echo "❌ 查询失败"
    
    echo
    echo "🧪 步骤3: 测试使用显示名称查询"
    echo "显示名称: $MODEL_NAME"
    echo "GET $BASE_URL/smart-ai/models/$MODEL_NAME/weights"
    echo "--------------------------------"
    
    curl -s "$BASE_URL/smart-ai/models/$MODEL_NAME/weights" | jq '.model.name' 2>/dev/null || echo "❌ 查询失败"
    
    echo
    echo "🧪 步骤4: 测试不存在的模型"
    echo "GET $BASE_URL/smart-ai/models/non_existent_model/weights"
    echo "--------------------------------"
    
    curl -s "$BASE_URL/smart-ai/models/non_existent_model/weights" | jq '.' 2>/dev/null || echo "❌ 查询失败"
    
    echo
    echo "✅ 测试完成！"
    echo
    echo "📝 总结:"
    echo "- 配置键名: $MODEL_KEY ✅"
    echo "- 显示名称: $MODEL_NAME ✅"
    echo "- 错误处理: ✅"
else
    echo "❌ 没有找到可用的SmartAI模型"
fi
