#!/bin/bash

# Interactive Chat API Example
# Demonstrates how to use the session-based chat API

API_URL="http://127.0.0.1:3000/api"

echo "🤖 SnapRAG Interactive Chat API Demo"
echo "======================================"
echo

# Step 1: Create a chat session
echo "📝 Step 1: Creating chat session with @jesse.base.eth..."
echo

# Check if server is running
if ! curl -s "$API_URL/health" > /dev/null 2>&1; then
    echo "❌ API server is not running at $API_URL"
    echo
    echo "Please start the server first:"
    echo "  snaprag serve api"
    echo
    exit 1
fi

SESSION_RESPONSE=$(curl -s -X POST "$API_URL/chat/create" \
  -H "Content-Type: application/json" \
  -d '{
    "user": "@jesse.base.eth",
    "context_limit": 20,
    "temperature": 0.7
  }')

echo "Response:"
echo "$SESSION_RESPONSE" | jq '.'
echo

# Extract session ID
SESSION_ID=$(echo "$SESSION_RESPONSE" | jq -r '.data.session_id // empty')

if [ -z "$SESSION_ID" ] || [ "$SESSION_ID" == "null" ]; then
    echo "❌ Failed to create session"
    echo "Error: $(echo "$SESSION_RESPONSE" | jq -r '.error // "Unknown error"')"
    exit 1
fi

echo "✅ Session created: $SESSION_ID"
echo

# Step 2: Send first message
echo "💬 Step 2: Sending first message..."
echo

MESSAGE_1=$(curl -s -X POST "$API_URL/chat/message" \
  -H "Content-Type: application/json" \
  -d "{
    \"session_id\": \"$SESSION_ID\",
    \"message\": \"What are your thoughts on building on Base?\"
  }")

echo "Response:"
echo "$MESSAGE_1" | jq -r '.data.message'
echo
echo "───────────────────────────────────────────────────────────────"
echo

# Step 3: Send follow-up message (with context)
echo "💬 Step 3: Sending follow-up message (context-aware)..."
echo

MESSAGE_2=$(curl -s -X POST "$API_URL/chat/message" \
  -H "Content-Type: application/json" \
  -d "{
    \"session_id\": \"$SESSION_ID\",
    \"message\": \"What excites you most about it?\"
  }")

echo "Response:"
echo "$MESSAGE_2" | jq -r '.data.message'
echo
echo "───────────────────────────────────────────────────────────────"
echo

# Step 4: Get session info
echo "📊 Step 4: Getting session information..."
echo

SESSION_INFO=$(curl -s "$API_URL/chat/session?session_id=$SESSION_ID")
echo "$SESSION_INFO" | jq '.data | {
  session_id,
  fid,
  username,
  display_name,
  conversation_length: (.conversation_history | length),
  last_activity
}'
echo

# Step 5: View conversation history
echo "📜 Step 5: Full conversation history..."
echo
echo "$SESSION_INFO" | jq -r '.data.conversation_history[] | "\(.role | ascii_upcase): \(.content)\n"'

# Step 6: Delete session
echo "🗑️  Step 6: Deleting session..."
curl -s -X DELETE "$API_URL/chat/session/$SESSION_ID" | jq '.'
echo

echo "✅ Demo complete!"
echo
echo "💡 Try it yourself:"
echo "  # Create session"
echo "  curl -X POST $API_URL/chat/create -H 'Content-Type: application/json' -d '{\"user\":\"99\"}'"
echo
echo "  # Send message"
echo "  curl -X POST $API_URL/chat/message -H 'Content-Type: application/json' -d '{\"session_id\":\"xxx\",\"message\":\"your question\"}'"
echo

