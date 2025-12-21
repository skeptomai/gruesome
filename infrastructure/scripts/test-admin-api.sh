#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print test result
pass() {
    echo -e "${GREEN}✓ $1${NC}"
}

fail() {
    echo -e "${RED}✗ $1${NC}"
    echo -e "${RED}  $2${NC}"
}

info() {
    echo -e "${BLUE}→ $1${NC}"
}

warn() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Parse command line arguments
ENVIRONMENT=${1:-production}

if [ "$ENVIRONMENT" != "production" ] && [ "$ENVIRONMENT" != "staging" ]; then
    echo "Usage: $0 [production|staging]"
    echo ""
    echo "Examples:"
    echo "  TEST_ADMIN_USERNAME=admin TEST_ADMIN_PASSWORD=pass $0"
    echo "  TEST_ADMIN_USERNAME=admin TEST_ADMIN_PASSWORD=pass $0 production"
    echo "  TEST_ADMIN_USERNAME=admin TEST_ADMIN_PASSWORD=pass $0 staging"
    echo ""
    echo "Required environment variables:"
    echo "  TEST_ADMIN_USERNAME - Admin username"
    echo "  TEST_ADMIN_PASSWORD - Admin password"
    echo ""
    echo "Optional environment variables:"
    echo "  TEST_USER_USERNAME - Non-admin username for authorization testing"
    echo "  TEST_USER_PASSWORD - Non-admin password for authorization testing"
    exit 1
fi

# Set API endpoint based on environment
if [ "$ENVIRONMENT" = "staging" ]; then
    API_ENDPOINT="https://api-staging.gruesome.skeptomai.com"
else
    API_ENDPOINT="https://api.gruesome.skeptomai.com"
fi

echo "========================================="
echo "Admin API Endpoint Tests"
echo "========================================="
echo ""
info "Environment: $ENVIRONMENT"
info "API Endpoint: $API_ENDPOINT"
echo ""

# Check required credentials
if [ -z "$TEST_ADMIN_USERNAME" ] || [ -z "$TEST_ADMIN_PASSWORD" ]; then
    fail "Missing credentials" "Set TEST_ADMIN_USERNAME and TEST_ADMIN_PASSWORD environment variables"
    exit 1
fi

# Step 1: Authenticate as admin user
info "1. Authenticating as admin user ($TEST_ADMIN_USERNAME)..."
LOGIN_RESPONSE=$(curl -s -X POST "$API_ENDPOINT/api/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"$TEST_ADMIN_USERNAME\",\"password\":\"$TEST_ADMIN_PASSWORD\"}" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$LOGIN_RESPONSE" | tail -n1)
RESPONSE_BODY=$(echo "$LOGIN_RESPONSE" | head -n-1)

if [ "$HTTP_CODE" = "200" ]; then
    ACCESS_TOKEN=$(echo "$RESPONSE_BODY" | jq -r '.access_token' 2>/dev/null || echo "")
    if [ -n "$ACCESS_TOKEN" ] && [ "$ACCESS_TOKEN" != "null" ]; then
        pass "Authentication successful"
    else
        fail "Authentication failed" "No access token in response: $RESPONSE_BODY"
        exit 1
    fi
else
    fail "Authentication failed (HTTP $HTTP_CODE)" "$RESPONSE_BODY"
    exit 1
fi
echo ""

# Step 2: Test GET /api/admin/games (list all games)
info "2. Testing GET /api/admin/games (list all games)..."
LIST_RESPONSE=$(curl -s -X GET "$API_ENDPOINT/api/admin/games" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$LIST_RESPONSE" | tail -n1)
RESPONSE_BODY=$(echo "$LIST_RESPONSE" | head -n-1)

if [ "$HTTP_CODE" = "200" ]; then
    GAME_COUNT=$(echo "$RESPONSE_BODY" | jq -r '.total' 2>/dev/null || echo "0")
    pass "List games endpoint working (found $GAME_COUNT games)"

    # Display first 3 games
    echo "$RESPONSE_BODY" | jq -r '.games[:3][] | "  - \(.game_id): \(.title) by \(.author)"' 2>/dev/null || true
else
    fail "List games failed (HTTP $HTTP_CODE)" "$RESPONSE_BODY"
fi
echo ""

# Step 3: Test POST /api/admin/games/upload-url
info "3. Testing POST /api/admin/games/upload-url (generate presigned URL)..."
UPLOAD_URL_RESPONSE=$(curl -s -X POST "$API_ENDPOINT/api/admin/games/upload-url" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"filename":"test-game.z3"}' \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$UPLOAD_URL_RESPONSE" | tail -n1)
RESPONSE_BODY=$(echo "$UPLOAD_URL_RESPONSE" | head -n-1)

if [ "$HTTP_CODE" = "200" ]; then
    S3_KEY=$(echo "$RESPONSE_BODY" | jq -r '.s3_key' 2>/dev/null || echo "")
    UPLOAD_URL=$(echo "$RESPONSE_BODY" | jq -r '.upload_url' 2>/dev/null || echo "")
    EXPIRES_IN=$(echo "$RESPONSE_BODY" | jq -r '.expires_in' 2>/dev/null || echo "")

    if [ -n "$S3_KEY" ] && [ "$S3_KEY" != "null" ]; then
        pass "Upload URL generated successfully"
        echo "  S3 Key: $S3_KEY"
        echo "  Expires in: $EXPIRES_IN seconds"
    else
        fail "Upload URL response missing s3_key" "$RESPONSE_BODY"
    fi
else
    fail "Upload URL generation failed (HTTP $HTTP_CODE)" "$RESPONSE_BODY"
fi
echo ""

# Step 4: Test GET /api/admin/games/{id} (get specific game)
if [ "$GAME_COUNT" -gt 0 ]; then
    FIRST_GAME_ID=$(echo "$RESPONSE_BODY" | jq -r '.games[0].game_id' 2>/dev/null || echo "")

    if [ -n "$FIRST_GAME_ID" ] && [ "$FIRST_GAME_ID" != "null" ]; then
        info "4. Testing GET /api/admin/games/{id} (game: $FIRST_GAME_ID)..."
        GET_GAME_RESPONSE=$(curl -s -X GET "$API_ENDPOINT/api/admin/games/$FIRST_GAME_ID" \
            -H "Authorization: Bearer $ACCESS_TOKEN" \
            -w "\n%{http_code}")

        HTTP_CODE=$(echo "$GET_GAME_RESPONSE" | tail -n1)
        RESPONSE_BODY=$(echo "$GET_GAME_RESPONSE" | head -n-1)

        if [ "$HTTP_CODE" = "200" ]; then
            TITLE=$(echo "$RESPONSE_BODY" | jq -r '.title' 2>/dev/null || echo "")
            AUTHOR=$(echo "$RESPONSE_BODY" | jq -r '.author' 2>/dev/null || echo "")
            VERSION=$(echo "$RESPONSE_BODY" | jq -r '.version' 2>/dev/null || echo "")

            pass "Get game endpoint working"
            echo "  Title: $TITLE"
            echo "  Author: $AUTHOR"
            echo "  Version: $VERSION"
        else
            fail "Get game failed (HTTP $HTTP_CODE)" "$RESPONSE_BODY"
        fi
    else
        warn "Skipping GET /api/admin/games/{id} test (no games found)"
    fi
else
    warn "Skipping GET /api/admin/games/{id} test (no games in database)"
fi
echo ""

# Step 5: Test unauthorized access (without admin role)
if [ -n "$TEST_USER_USERNAME" ] && [ -n "$TEST_USER_PASSWORD" ]; then
    info "5. Testing unauthorized access (non-admin user: $TEST_USER_USERNAME)..."
    USER_LOGIN=$(curl -s -X POST "$API_ENDPOINT/api/auth/login" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$TEST_USER_USERNAME\",\"password\":\"$TEST_USER_PASSWORD\"}" \
        -w "\n%{http_code}")

    HTTP_CODE=$(echo "$USER_LOGIN" | tail -n1)
    RESPONSE_BODY=$(echo "$USER_LOGIN" | head -n-1)

    if [ "$HTTP_CODE" = "200" ]; then
        USER_TOKEN=$(echo "$RESPONSE_BODY" | jq -r '.access_token' 2>/dev/null || echo "")

        if [ -n "$USER_TOKEN" ] && [ "$USER_TOKEN" != "null" ]; then
            FORBIDDEN_RESPONSE=$(curl -s -X GET "$API_ENDPOINT/api/admin/games" \
                -H "Authorization: Bearer $USER_TOKEN" \
                -w "\n%{http_code}")

            HTTP_CODE=$(echo "$FORBIDDEN_RESPONSE" | tail -n1)

            if [ "$HTTP_CODE" = "403" ]; then
                pass "Authorization check working (non-admin user correctly denied)"
            else
                fail "Authorization check failed" "Expected HTTP 403, got $HTTP_CODE"
            fi
        else
            warn "Non-admin user login failed - skipping authorization test"
        fi
    else
        warn "Non-admin user login failed (HTTP $HTTP_CODE) - skipping authorization test"
    fi
else
    info "5. Skipping unauthorized access test (set TEST_USER_USERNAME and TEST_USER_PASSWORD)"
fi
echo ""

# Summary
echo "========================================="
echo -e "${GREEN}Admin API Tests Complete${NC}"
echo "========================================="
echo ""
echo "Tested endpoints:"
echo "  ✓ POST /api/auth/login (admin authentication)"
echo "  ✓ GET /api/admin/games (list all games)"
echo "  ✓ POST /api/admin/games/upload-url (generate presigned URL)"
if [ "$GAME_COUNT" -gt 0 ]; then
    echo "  ✓ GET /api/admin/games/{id} (get specific game)"
fi
if [ -n "$TEST_USER_USERNAME" ] && [ -n "$TEST_USER_PASSWORD" ]; then
    echo "  ✓ Authorization check (non-admin user)"
fi
echo ""
echo "Note: The following endpoints were not tested (require test data):"
echo "  - PUT /api/admin/games/{id} (update game metadata)"
echo "  - POST /api/admin/games (create new game metadata)"
echo "  - DELETE /api/admin/games/{id} (soft delete game)"
echo ""
