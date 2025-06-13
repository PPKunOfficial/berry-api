#!/bin/bash

# 流式请求错误处理测试脚本
# 用于验证流式请求失败时返回正确的HTTP状态码

BASE_URL="http://localhost:3000"

echo "🧪 流式请求错误处理测试"
echo "========================"

# 检查服务是否运行
echo "📡 检查服务状态..."
if ! curl -s "$BASE_URL/health" > /dev/null; then
    echo "❌ 服务未运行，请先启动Berry API服务"
    exit 1
fi
echo "✅ 服务正常运行"
echo

# 测试1: 使用无效token的流式请求
echo "🧪 测试1: 无效token的流式请求"
echo "预期: 返回HTTP错误状态码，而不是200 + SSE错误"
echo "----------------------------------------"

RESPONSE=$(curl -s -w "HTTPSTATUS:%{http_code}" \
  -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer invalid-token-12345" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
  }')

HTTP_STATUS=$(echo "$RESPONSE" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | sed 's/HTTPSTATUS:[0-9]*$//')

echo "HTTP状态码: $HTTP_STATUS"
echo "响应体:"
echo "$BODY" | jq '.' 2>/dev/null || echo "$BODY"

if [ "$HTTP_STATUS" = "200" ]; then
    echo "❌ 错误：仍然返回HTTP 200状态码"
    echo "   应该返回错误状态码（如403、500、503等）"
else
    echo "✅ 正确：返回HTTP错误状态码 $HTTP_STATUS"
fi
echo
echo

# 测试2: 使用不存在模型的流式请求
echo "🧪 测试2: 不存在模型的流式请求"
echo "----------------------------------------"

RESPONSE=$(curl -s -w "HTTPSTATUS:%{http_code}" \
  -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer test-token" \
  -d '{
    "model": "non-existent-model-12345",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
  }')

HTTP_STATUS=$(echo "$RESPONSE" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | sed 's/HTTPSTATUS:[0-9]*$//')

echo "HTTP状态码: $HTTP_STATUS"
echo "响应体:"
echo "$BODY" | jq '.' 2>/dev/null || echo "$BODY"

if [ "$HTTP_STATUS" = "200" ]; then
    echo "❌ 错误：仍然返回HTTP 200状态码"
else
    echo "✅ 正确：返回HTTP错误状态码 $HTTP_STATUS"
fi
echo
echo

# 测试3: 对比非流式请求的错误处理
echo "🧪 测试3: 非流式请求错误处理（对比）"
echo "----------------------------------------"

RESPONSE=$(curl -s -w "HTTPSTATUS:%{http_code}" \
  -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer invalid-token-12345" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": false
  }')

HTTP_STATUS=$(echo "$RESPONSE" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | sed 's/HTTPSTATUS:[0-9]*$//')

echo "HTTP状态码: $HTTP_STATUS"
echo "响应体:"
echo "$BODY" | jq '.' 2>/dev/null || echo "$BODY"

if [ "$HTTP_STATUS" = "200" ]; then
    echo "❌ 错误：非流式请求也返回HTTP 200状态码"
else
    echo "✅ 正确：非流式请求返回HTTP错误状态码 $HTTP_STATUS"
fi
echo
echo

# 测试4: 正常的流式请求（确保修复没有破坏正常功能）
echo "🧪 测试4: 正常流式请求（验证正常功能）"
echo "----------------------------------------"

# 首先获取可用的模型
MODELS_RESPONSE=$(curl -s "$BASE_URL/models" -H "Authorization: Bearer test-token")
FIRST_MODEL=$(echo "$MODELS_RESPONSE" | jq -r '.data[0].id' 2>/dev/null)

if [ "$FIRST_MODEL" != "null" ] && [ -n "$FIRST_MODEL" ]; then
    echo "使用模型: $FIRST_MODEL"
    
    # 发送正常的流式请求（只读取前几行）
    timeout 5s curl -s \
      -X POST "$BASE_URL/v1/chat/completions" \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer test-token" \
      -d "{
        \"model\": \"$FIRST_MODEL\",
        \"messages\": [{\"role\": \"user\", \"content\": \"Say hello\"}],
        \"stream\": true,
        \"max_tokens\": 10
      }" | head -n 5
    
    echo
    echo "✅ 正常流式请求测试完成"
else
    echo "⚠️  无法获取可用模型，跳过正常功能测试"
fi
echo
echo

# 总结
echo "📋 测试总结"
echo "=========="
echo "修复验证要点："
echo "1. ✅ 流式请求失败时应返回HTTP错误状态码（非200）"
echo "2. ✅ 错误响应应为JSON格式，包含错误信息"
echo "3. ✅ 不应在SSE流中包含错误信息"
echo "4. ✅ 正常的流式请求功能不受影响"
echo
echo "💡 如果看到HTTP 200状态码配合错误信息，说明修复未生效"
echo "💡 正确的行为是返回4xx/5xx状态码配合JSON错误响应"
