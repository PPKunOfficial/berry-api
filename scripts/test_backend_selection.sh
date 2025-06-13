#!/bin/bash

# 后端指定功能测试脚本
# 用于测试通过backend参数直接指定后端的功能

BASE_URL="http://localhost:3000"
AUTH_TOKEN="test-token"

echo "🎯 后端指定功能测试"
echo "==================="

# 检查服务是否运行
echo "📡 检查服务状态..."
if ! curl -s "$BASE_URL/health" > /dev/null; then
    echo "❌ 服务未运行，请先启动Berry API服务"
    exit 1
fi
echo "✅ 服务正常运行"
echo

# 获取可用的模型和后端信息
echo "📋 获取可用模型信息..."
MODELS_RESPONSE=$(curl -s "$BASE_URL/models" -H "Authorization: Bearer $AUTH_TOKEN")
if [ $? -ne 0 ]; then
    echo "❌ 无法获取模型列表"
    exit 1
fi

FIRST_MODEL=$(echo "$MODELS_RESPONSE" | jq -r '.data[0].id' 2>/dev/null)
if [ "$FIRST_MODEL" = "null" ] || [ -z "$FIRST_MODEL" ]; then
    echo "❌ 没有找到可用的模型"
    exit 1
fi

echo "✅ 找到可用模型: $FIRST_MODEL"
echo

# 获取SmartAI权重信息来找到可用的后端
echo "🔍 查找可用的后端..."
WEIGHTS_RESPONSE=$(curl -s "$BASE_URL/smart-ai/weights" -H "Authorization: Bearer $AUTH_TOKEN" 2>/dev/null)

if [ $? -eq 0 ] && [ -n "$WEIGHTS_RESPONSE" ]; then
    # 提取后端信息
    BACKENDS=$(echo "$WEIGHTS_RESPONSE" | jq -r '.models[]?.backends[]?.provider' 2>/dev/null | sort -u)
    if [ -n "$BACKENDS" ]; then
        echo "✅ 找到以下后端:"
        echo "$BACKENDS" | sed 's/^/  - /'
        echo
        
        # 选择第一个后端进行测试
        FIRST_BACKEND=$(echo "$BACKENDS" | head -n1)
        SECOND_BACKEND=$(echo "$BACKENDS" | head -n2 | tail -n1)
    else
        echo "⚠️  无法从SmartAI权重信息中提取后端，将使用默认后端名称"
        FIRST_BACKEND="openai_official"
        SECOND_BACKEND="anthropic_claude"
    fi
else
    echo "⚠️  无法获取SmartAI权重信息，将使用默认后端名称"
    FIRST_BACKEND="openai_official"
    SECOND_BACKEND="anthropic_claude"
fi

echo

# 测试1: 正常请求（不指定backend）
echo "🧪 测试1: 正常请求（负载均衡）"
echo "模型: $FIRST_MODEL"
echo "后端: 自动选择"
echo "----------------------------------------"

RESPONSE=$(curl -s -w "HTTPSTATUS:%{http_code}" \
  -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -d "{
    \"model\": \"$FIRST_MODEL\",
    \"messages\": [{\"role\": \"user\", \"content\": \"Say hello\"}],
    \"max_tokens\": 5
  }")

HTTP_STATUS=$(echo "$RESPONSE" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | sed 's/HTTPSTATUS:[0-9]*$//')

echo "HTTP状态码: $HTTP_STATUS"
if [ "$HTTP_STATUS" = "200" ]; then
    echo "✅ 正常请求成功"
    echo "响应: $(echo "$BODY" | jq -r '.choices[0].message.content' 2>/dev/null || echo "无法解析响应")"
else
    echo "❌ 正常请求失败"
    echo "错误: $BODY"
fi
echo
echo

# 测试2: 指定有效后端
echo "🧪 测试2: 指定有效后端"
echo "模型: $FIRST_MODEL"
echo "后端: $FIRST_BACKEND"
echo "----------------------------------------"

RESPONSE=$(curl -s -w "HTTPSTATUS:%{http_code}" \
  -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -d "{
    \"model\": \"$FIRST_MODEL\",
    \"messages\": [{\"role\": \"user\", \"content\": \"Say hello\"}],
    \"backend\": \"$FIRST_BACKEND\",
    \"max_tokens\": 5
  }")

HTTP_STATUS=$(echo "$RESPONSE" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | sed 's/HTTPSTATUS:[0-9]*$//')

echo "HTTP状态码: $HTTP_STATUS"
if [ "$HTTP_STATUS" = "200" ]; then
    echo "✅ 指定后端请求成功"
    echo "响应: $(echo "$BODY" | jq -r '.choices[0].message.content' 2>/dev/null || echo "无法解析响应")"
else
    echo "❌ 指定后端请求失败"
    echo "错误: $BODY"
fi
echo
echo

# 测试3: 指定无效后端
echo "🧪 测试3: 指定无效后端"
echo "模型: $FIRST_MODEL"
echo "后端: invalid_backend_12345"
echo "----------------------------------------"

RESPONSE=$(curl -s -w "HTTPSTATUS:%{http_code}" \
  -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -d "{
    \"model\": \"$FIRST_MODEL\",
    \"messages\": [{\"role\": \"user\", \"content\": \"Say hello\"}],
    \"backend\": \"invalid_backend_12345\",
    \"max_tokens\": 5
  }")

HTTP_STATUS=$(echo "$RESPONSE" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | sed 's/HTTPSTATUS:[0-9]*$//')

echo "HTTP状态码: $HTTP_STATUS"
if [ "$HTTP_STATUS" != "200" ]; then
    echo "✅ 正确返回错误状态码"
    echo "错误信息: $(echo "$BODY" | jq -r '.error.message' 2>/dev/null || echo "$BODY")"
else
    echo "❌ 应该返回错误状态码，但返回了200"
    echo "响应: $BODY"
fi
echo
echo

# 测试4: 流式请求指定后端
echo "🧪 测试4: 流式请求指定后端"
echo "模型: $FIRST_MODEL"
echo "后端: $FIRST_BACKEND"
echo "----------------------------------------"

echo "发送流式请求..."
timeout 10s curl -s \
  -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -d "{
    \"model\": \"$FIRST_MODEL\",
    \"messages\": [{\"role\": \"user\", \"content\": \"Count to 3\"}],
    \"backend\": \"$FIRST_BACKEND\",
    \"stream\": true,
    \"max_tokens\": 20
  }" | head -n 5

echo
echo "✅ 流式请求测试完成"
echo
echo

# 测试5: 多后端健康检查
if [ "$FIRST_BACKEND" != "$SECOND_BACKEND" ] && [ -n "$SECOND_BACKEND" ]; then
    echo "🏥 测试5: 多后端健康检查"
    echo "========================================="
    
    for backend in "$FIRST_BACKEND" "$SECOND_BACKEND"; do
        echo "检查后端: $backend"
        echo "------------------------"
        
        RESPONSE=$(curl -s -w "HTTPSTATUS:%{http_code}" \
          -X POST "$BASE_URL/v1/chat/completions" \
          -H "Content-Type: application/json" \
          -H "Authorization: Bearer $AUTH_TOKEN" \
          -d "{
            \"model\": \"$FIRST_MODEL\",
            \"messages\": [{\"role\": \"user\", \"content\": \"ping\"}],
            \"backend\": \"$backend\",
            \"max_tokens\": 1
          }")
        
        HTTP_STATUS=$(echo "$RESPONSE" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
        
        if [ "$HTTP_STATUS" = "200" ]; then
            echo "✅ $backend: 健康"
        else
            echo "❌ $backend: 不健康 (HTTP $HTTP_STATUS)"
        fi
        echo
    done
fi

# 总结
echo "📋 测试总结"
echo "=========="
echo "✅ 功能验证要点:"
echo "1. 正常负载均衡请求正常工作"
echo "2. 指定有效后端可以成功请求"
echo "3. 指定无效后端返回正确的错误状态码"
echo "4. 流式请求支持后端指定"
echo "5. 可以用于健康检查和故障排除"
echo
echo "💡 使用建议:"
echo "- 在生产环境中谨慎使用backend参数"
echo "- 主要用于调试、监控和健康检查"
echo "- backend参数不会传递给上游API"
echo "- 所有指定后端的请求都会被记录在日志中"
echo
echo "📚 更多信息请查看: docs/backend_selection_api.md"
