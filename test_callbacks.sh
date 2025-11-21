#!/bin/bash

# Timer Platform - Callback Test Script
# Tests both HTTP and NATS callback functionality

set -e

API_KEY="dev-api-key-change-in-production-min-32-chars"
BASE_URL="http://localhost:8080"

echo "=== Timer Platform Callback Test Suite ==="
echo ""

# Get current UTC time
CURRENT_TIME=$(curl -s $BASE_URL/healthz | jq -r '.data.timestamp')
echo "Current UTC time: $CURRENT_TIME"
echo ""

# Calculate future times (1 and 2 minutes from now)
FUTURE_1MIN=$(date -u -v+1M +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u -d "+1 minute" +"%Y-%m-%dT%H:%M:%SZ")
FUTURE_2MIN=$(date -u -v+2M +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u -d "+2 minutes" +"%Y-%m-%dT%H:%M:%SZ")

echo "Test timer 1 (HTTP): $FUTURE_1MIN"
echo "Test timer 2 (NATS): $FUTURE_2MIN"
echo ""

# Test 1: Create HTTP callback timer
echo "=== Test 1: Create HTTP Callback Timer ==="
HTTP_RESPONSE=$(curl -s -X POST $BASE_URL/timers \
  -H "X-API-Key: $API_KEY" \
  -H "Content-Type: application/json" \
  -d "{
    \"execute_at\": \"$FUTURE_1MIN\",
    \"callback\": {
      \"type\": \"http\",
      \"url\": \"https://httpbin.org/post\",
      \"headers\": {\"X-Test-Header\": \"http-test\"},
      \"payload\": {\"event\": \"http_callback_test\", \"timestamp\": \"$FUTURE_1MIN\"}
    }
  }")

HTTP_TIMER_ID=$(echo $HTTP_RESPONSE | jq -r '.data.id')
echo "HTTP Timer Created: $HTTP_TIMER_ID"
echo $HTTP_RESPONSE | jq .
echo ""

# Test 2: Create NATS callback timer
echo "=== Test 2: Create NATS Callback Timer ==="
NATS_RESPONSE=$(curl -s -X POST $BASE_URL/timers \
  -H "X-API-Key: $API_KEY" \
  -H "Content-Type: application/json" \
  -d "{
    \"execute_at\": \"$FUTURE_2MIN\",
    \"callback\": {
      \"type\": \"nats\",
      \"topic\": \"timers.test.execution\",
      \"key\": \"test-key-123\",
      \"payload\": {\"event\": \"nats_callback_test\", \"timestamp\": \"$FUTURE_2MIN\"}
    }
  }")

NATS_TIMER_ID=$(echo $NATS_RESPONSE | jq -r '.data.id')
echo "NATS Timer Created: $NATS_TIMER_ID"
echo $NATS_RESPONSE | jq .
echo ""

# Test 3: Get timer details
echo "=== Test 3: Get Timer Details ==="
echo "HTTP Timer Details:"
curl -s -X GET $BASE_URL/timers/$HTTP_TIMER_ID \
  -H "X-API-Key: $API_KEY" | jq '.data | {id, callback_type: .callback.type, url: .callback.url, status}'
echo ""

echo "NATS Timer Details:"
curl -s -X GET $BASE_URL/timers/$NATS_TIMER_ID \
  -H "X-API-Key: $API_KEY" | jq '.data | {id, callback_type: .callback.type, topic: .callback.topic, key: .callback.key, status}'
echo ""

# Test 4: List all timers
echo "=== Test 4: List All Timers ==="
curl -s -X GET "$BASE_URL/timers?limit=5" \
  -H "X-API-Key: $API_KEY" | jq '.data | {total, timers: .timers | map({id, callback_type, status})}'
echo ""

# Test 5: Filter by callback type
echo "=== Test 5: Filter by Callback Type ==="
echo "HTTP Timers:"
curl -s -X GET "$BASE_URL/timers?callback_type=http&limit=3" \
  -H "X-API-Key: $API_KEY" | jq '.data.total'

echo "NATS Timers:"
curl -s -X GET "$BASE_URL/timers?callback_type=nats&limit=3" \
  -H "X-API-Key: $API_KEY" | jq '.data.total'
echo ""

# Wait for execution
echo "=== Waiting for Timer Execution ==="
echo "Waiting 70 seconds for HTTP timer to execute..."
sleep 70

# Check HTTP timer execution
echo ""
echo "=== Test 6: Verify HTTP Timer Execution ==="
HTTP_RESULT=$(curl -s -X GET $BASE_URL/timers/$HTTP_TIMER_ID -H "X-API-Key: $API_KEY")
HTTP_STATUS=$(echo $HTTP_RESULT | jq -r '.data.status')
HTTP_EXECUTED_AT=$(echo $HTTP_RESULT | jq -r '.data.executed_at')

if [ "$HTTP_STATUS" = "completed" ]; then
  echo "✅ HTTP Timer Executed Successfully"
  echo "   Status: $HTTP_STATUS"
  echo "   Executed at: $HTTP_EXECUTED_AT"
else
  echo "❌ HTTP Timer Execution Failed"
  echo "   Status: $HTTP_STATUS"
  echo "   Error: $(echo $HTTP_RESULT | jq -r '.data.last_error')"
fi
echo ""

# Wait for NATS timer
echo "Waiting 60 more seconds for NATS timer to execute..."
sleep 60

# Check NATS timer execution
echo ""
echo "=== Test 7: Verify NATS Timer Execution ==="
NATS_RESULT=$(curl -s -X GET $BASE_URL/timers/$NATS_TIMER_ID -H "X-API-Key: $API_KEY")
NATS_STATUS=$(echo $NATS_RESULT | jq -r '.data.status')
NATS_EXECUTED_AT=$(echo $NATS_RESULT | jq -r '.data.executed_at')

if [ "$NATS_STATUS" = "completed" ]; then
  echo "✅ NATS Timer Executed Successfully"
  echo "   Status: $NATS_STATUS"
  echo "   Executed at: $NATS_EXECUTED_AT"
else
  echo "❌ NATS Timer Execution Failed"
  echo "   Status: $NATS_STATUS"
  echo "   Error: $(echo $NATS_RESULT | jq -r '.data.last_error')"
fi
echo ""

# Summary
echo "=== Test Summary ==="
echo "HTTP Timer ID: $HTTP_TIMER_ID - Status: $HTTP_STATUS"
echo "NATS Timer ID: $NATS_TIMER_ID - Status: $NATS_STATUS"
echo ""

# Check scheduler logs
echo "=== Scheduler Logs (last 20 lines) ==="
docker-compose logs timer | grep -E "(callback|Loaded|Spawned)" | tail -20

echo ""
echo "=== Test Suite Complete ==="
