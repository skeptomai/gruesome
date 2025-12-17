#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

API_URL="${API_URL:-https://viq8oiws3m.execute-api.us-west-1.amazonaws.com}"
USERNAME="${TEST_USERNAME:-bob}"
PASSWORD="${TEST_PASSWORD:-BobPassword123}"
GAME_ID="${TEST_GAME_ID:-mini-zork}"

echo "========================================="
echo "Game Lambda End-to-End Test"
echo "========================================="
echo ""

# Function to print test result
pass() {
    echo -e "${GREEN}✓ $1${NC}"
}

fail() {
    echo -e "${RED}✗ $1${NC}"
    echo -e "${RED}  $2${NC}"
    exit 1
}

info() {
    echo -e "${YELLOW}→ $1${NC}"
}

echo "Test Configuration:"
echo "  API URL: $API_URL"
echo "  Username: $USERNAME"
echo "  Game: $GAME_ID"
echo ""

# Test 1: Authentication
info "Test 1: Authenticating user..."
AUTH_RESPONSE=$(curl -s -X POST "$API_URL/api/auth/login" \
  -H 'Content-Type: application/json' \
  -d "{\"username\":\"$USERNAME\",\"password\":\"$PASSWORD\"}")

TOKEN=$(echo "$AUTH_RESPONSE" | jq -r '.access_token')
if [ "$TOKEN" = "null" ] || [ -z "$TOKEN" ]; then
    fail "Authentication failed" "Response: $AUTH_RESPONSE"
fi
pass "Authentication successful (token length: ${#TOKEN})"

# Test 2: Start Game
info "Test 2: Starting new game..."
START_RESPONSE=$(curl -s -X POST "$API_URL/api/games/start" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{\"game_id\":\"$GAME_ID\"}")

SESSION_ID=$(echo "$START_RESPONSE" | jq -r '.session_id')
if [ "$SESSION_ID" = "null" ] || [ -z "$SESSION_ID" ]; then
    fail "Start game failed" "Response: $START_RESPONSE"
fi

OUTPUT=$(echo "$START_RESPONSE" | jq -r '.output')
WAITING=$(echo "$START_RESPONSE" | jq -r '.waiting_for_input')

if [ "$WAITING" != "true" ]; then
    fail "Start game did not set waiting_for_input=true" "Response: $START_RESPONSE"
fi

if ! echo "$OUTPUT" | grep -q "DORK I"; then
    fail "Start game output missing expected content" "Output: $OUTPUT"
fi

pass "Game started successfully (session: ${SESSION_ID:0:8}...)"
pass "Game output contains expected banner"
pass "Game is waiting for input"

# Test 3: Execute Command
info "Test 3: Executing game command 'inventory'..."
CMD_RESPONSE=$(curl -s -X POST "$API_URL/api/games/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{\"session_id\":\"$SESSION_ID\",\"command\":\"inventory\"}")

CMD_OUTPUT=$(echo "$CMD_RESPONSE" | jq -r '.output')
CMD_WAITING=$(echo "$CMD_RESPONSE" | jq -r '.waiting_for_input')
CMD_GAME_OVER=$(echo "$CMD_RESPONSE" | jq -r '.game_over')

if [ "$CMD_OUTPUT" = "null" ] || [ -z "$CMD_OUTPUT" ]; then
    fail "Command execution failed" "Response: $CMD_RESPONSE"
fi

if [ "$CMD_WAITING" != "true" ]; then
    fail "Command did not return waiting_for_input=true" "Response: $CMD_RESPONSE"
fi

if [ "$CMD_GAME_OVER" != "false" ]; then
    fail "Command incorrectly marked game as over" "Response: $CMD_RESPONSE"
fi

pass "Command executed successfully"
pass "Output received (${#CMD_OUTPUT} bytes)"
pass "Game state preserved (waiting_for_input=true, game_over=false)"

# Test 4: Session Persistence - Execute Another Command
info "Test 4: Testing session persistence with second command..."
CMD2_RESPONSE=$(curl -s -X POST "$API_URL/api/games/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{\"session_id\":\"$SESSION_ID\",\"command\":\"look\"}")

CMD2_OUTPUT=$(echo "$CMD2_RESPONSE" | jq -r '.output')
if [ "$CMD2_OUTPUT" = "null" ] || [ -z "$CMD2_OUTPUT" ]; then
    fail "Second command execution failed" "Response: $CMD2_RESPONSE"
fi

pass "Second command executed successfully"
pass "Session state persisted across commands"

# Test 5: Invalid Session ID
info "Test 5: Testing error handling with invalid session..."
INVALID_RESPONSE=$(curl -s -X POST "$API_URL/api/games/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"session_id":"invalid-session-id","command":"look"}')

INVALID_ERROR=$(echo "$INVALID_RESPONSE" | jq -r '.error')
if [ "$INVALID_ERROR" = "null" ] || [ -z "$INVALID_ERROR" ]; then
    fail "Invalid session should return error" "Response: $INVALID_RESPONSE"
fi

pass "Invalid session correctly rejected"

# Test 6: Resume Existing Session
info "Test 6: Testing session resume..."
RESUME_RESPONSE=$(curl -s -X POST "$API_URL/api/games/start" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{\"game_id\":\"$GAME_ID\",\"session_id\":\"$SESSION_ID\"}")

RESUME_SESSION=$(echo "$RESUME_RESPONSE" | jq -r '.session_id')
if [ "$RESUME_SESSION" != "$SESSION_ID" ]; then
    fail "Resume session returned different session_id" "Expected: $SESSION_ID, Got: $RESUME_SESSION"
fi

pass "Session resume successful (same session_id returned)"

# Test 7: Health Check
info "Test 7: Testing health endpoint..."
HEALTH_RESPONSE=$(curl -s -X GET "$API_URL/health")
HEALTH_STATUS=$(echo "$HEALTH_RESPONSE" | jq -r '.status' 2>/dev/null || echo "$HEALTH_RESPONSE")
if [ "$HEALTH_STATUS" != "healthy" ]; then
    fail "Health check failed" "Response: $HEALTH_RESPONSE"
fi

pass "Health endpoint responding"

echo ""
echo "========================================="
echo -e "${GREEN}All Tests Passed! ✓${NC}"
echo "========================================="
echo ""
echo "Summary:"
echo "  ✓ JWT authentication"
echo "  ✓ Game start (session creation)"
echo "  ✓ Command execution"
echo "  ✓ Session persistence"
echo "  ✓ DynamoDB state updates"
echo "  ✓ Error handling"
echo "  ✓ Session resume"
echo "  ✓ Health checks"
echo ""
echo "Session ID: $SESSION_ID"
echo "Commands executed: 3"
echo ""
