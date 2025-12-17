#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "========================================="
echo "Infrastructure Verification"
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
    echo -e "${BLUE}→ $1${NC}"
}

warn() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Verify AWS CLI is available
if ! command -v aws &> /dev/null; then
    fail "AWS CLI not found" "Install with: brew install awscli"
fi

# Verify jq is available
if ! command -v jq &> /dev/null; then
    fail "jq not found" "Install with: brew install jq"
fi

# Get AWS account info
info "Checking AWS credentials..."
ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text 2>/dev/null || echo "")
if [ -z "$ACCOUNT_ID" ]; then
    fail "AWS credentials not configured" "Run: aws configure"
fi
pass "AWS credentials valid (Account: $ACCOUNT_ID)"

# Get region
REGION=$(aws configure get region || echo "us-west-1")
echo "  Region: $REGION"
echo ""

# Check CloudFormation stacks
info "Checking CloudFormation stacks..."

check_stack() {
    local stack_name=$1
    local status=$(aws cloudformation describe-stacks --stack-name "$stack_name" --query 'Stacks[0].StackStatus' --output text 2>/dev/null || echo "NOT_FOUND")

    if [ "$status" = "CREATE_COMPLETE" ] || [ "$status" = "UPDATE_COMPLETE" ]; then
        pass "Stack: $stack_name ($status)"
        return 0
    elif [ "$status" = "NOT_FOUND" ]; then
        warn "Stack not found: $stack_name"
        return 1
    else
        fail "Stack in bad state: $stack_name" "Status: $status"
        return 1
    fi
}

check_stack "GruesomeDataStack" || true
check_stack "GruesomeAuthStack" || true
check_stack "GruesomeBackendStack" || true
check_stack "GruesomeDnsStack" || true
check_stack "GruesomeFrontendStack" || true
echo ""

# Check DynamoDB table
info "Checking DynamoDB table..."
TABLE_NAME="gruesome-platform"
TABLE_STATUS=$(aws dynamodb describe-table --table-name "$TABLE_NAME" --query 'Table.TableStatus' --output text 2>/dev/null || echo "NOT_FOUND")

if [ "$TABLE_STATUS" = "ACTIVE" ]; then
    pass "DynamoDB table: $TABLE_NAME (ACTIVE)"

    # Check item count
    ITEM_COUNT=$(aws dynamodb scan --table-name "$TABLE_NAME" --select COUNT --query 'Count' --output text 2>/dev/null || echo "0")
    echo "  Items: $ITEM_COUNT"

    # Check TTL configuration
    TTL_STATUS=$(aws dynamodb describe-time-to-live --table-name "$TABLE_NAME" --query 'TimeToLiveDescription.TimeToLiveStatus' --output text 2>/dev/null || echo "UNKNOWN")
    if [ "$TTL_STATUS" = "ENABLED" ]; then
        pass "TTL enabled on $TABLE_NAME"
    else
        warn "TTL not enabled on $TABLE_NAME (Status: $TTL_STATUS)"
    fi
else
    fail "DynamoDB table issue" "Table: $TABLE_NAME, Status: $TABLE_STATUS"
fi
echo ""

# Check S3 buckets
info "Checking S3 buckets..."

check_bucket() {
    local bucket_name=$1
    if aws s3 ls "s3://$bucket_name" &>/dev/null; then
        pass "S3 bucket: $bucket_name"

        # Check if bucket has any objects
        OBJECT_COUNT=$(aws s3 ls "s3://$bucket_name" --recursive | wc -l)
        echo "  Objects: $OBJECT_COUNT"
        return 0
    else
        warn "S3 bucket not found or inaccessible: $bucket_name"
        return 1
    fi
}

check_bucket "gruesome-games" || true
check_bucket "gruesome-saves" || true
check_bucket "gruesome-frontend" || true
echo ""

# Check Cognito User Pool
info "Checking Cognito User Pool..."
USER_POOL_ID=$(aws cloudformation describe-stacks --stack-name GruesomeAuthStack --query 'Stacks[0].Outputs[?OutputKey==`UserPoolId`].OutputValue' --output text 2>/dev/null || echo "")

if [ -n "$USER_POOL_ID" ] && [ "$USER_POOL_ID" != "None" ]; then
    # Just check if we can describe it (Cognito pools don't have a simple "Status" field)
    if aws cognito-idp describe-user-pool --user-pool-id "$USER_POOL_ID" &>/dev/null; then
        pass "Cognito User Pool: $USER_POOL_ID"

        # Count users
        USER_COUNT=$(aws cognito-idp list-users --user-pool-id "$USER_POOL_ID" --query 'length(Users)' --output text 2>/dev/null || echo "0")
        echo "  Users: $USER_COUNT"
    else
        fail "Cannot access Cognito User Pool" "Pool: $USER_POOL_ID"
    fi
else
    warn "Could not find Cognito User Pool ID from stack outputs"
fi
echo ""

# Check Lambda functions
info "Checking Lambda functions..."

check_lambda() {
    local function_name_pattern=$1
    local description=$2

    FUNCTION_ARN=$(aws lambda list-functions --query "Functions[?contains(FunctionName, '$function_name_pattern')].FunctionArn | [0]" --output text 2>/dev/null || echo "")

    if [ -n "$FUNCTION_ARN" ] && [ "$FUNCTION_ARN" != "None" ]; then
        FUNCTION_NAME=$(echo "$FUNCTION_ARN" | awk -F: '{print $NF}')
        STATE=$(aws lambda get-function-configuration --function-name "$FUNCTION_NAME" --query 'State' --output text 2>/dev/null || echo "UNKNOWN")
        LAST_UPDATE=$(aws lambda get-function-configuration --function-name "$FUNCTION_NAME" --query 'LastUpdateStatus' --output text 2>/dev/null || echo "UNKNOWN")

        if [ "$STATE" = "Active" ] && [ "$LAST_UPDATE" = "Successful" ]; then
            pass "$description: $FUNCTION_NAME"
            echo "  State: $STATE, Last Update: $LAST_UPDATE"
        else
            warn "$description: $FUNCTION_NAME (State: $STATE, Update: $LAST_UPDATE)"
        fi
        return 0
    else
        warn "$description Lambda not found (pattern: *$function_name_pattern*)"
        return 1
    fi
}

check_lambda "AuthFunction" "Auth Lambda" || true
check_lambda "GameFunction" "Game Lambda" || true
echo ""

# Check API Gateway
info "Checking API Gateway..."
API_ID=$(aws apigatewayv2 get-apis --query "Items[?Name=='gruesome-api'].ApiId | [0]" --output text 2>/dev/null || echo "")

if [ -n "$API_ID" ] && [ "$API_ID" != "None" ]; then
    API_ENDPOINT=$(aws apigatewayv2 get-apis --query "Items[?ApiId=='$API_ID'].ApiEndpoint | [0]" --output text 2>/dev/null || echo "")
    pass "API Gateway: $API_ID"
    echo "  Endpoint: $API_ENDPOINT"

    # Check routes
    ROUTE_COUNT=$(aws apigatewayv2 get-routes --api-id "$API_ID" --query 'length(Items)' --output text 2>/dev/null || echo "0")
    echo "  Routes: $ROUTE_COUNT"

    # List key routes
    info "Checking critical routes..."
    ROUTES=$(aws apigatewayv2 get-routes --api-id "$API_ID" --query 'Items[].RouteKey' --output text 2>/dev/null || echo "")

    check_route() {
        local route=$1
        if echo "$ROUTES" | grep -q "$route"; then
            pass "Route exists: $route"
        else
            warn "Route missing: $route"
        fi
    }

    check_route "POST /api/auth/login"
    check_route "POST /api/games/start"
    check_route "POST /api/games/command"
    check_route "GET /health"
else
    warn "API Gateway not found"
fi
echo ""

# Check CloudFront distribution (if exists)
info "Checking CloudFront distribution..."
DISTRIBUTION_ID=$(aws cloudfront list-distributions --query "DistributionList.Items[?Comment=='Gruesome Frontend'].Id | [0]" --output text 2>/dev/null || echo "")

if [ -n "$DISTRIBUTION_ID" ] && [ "$DISTRIBUTION_ID" != "None" ]; then
    DIST_STATUS=$(aws cloudfront get-distribution --id "$DISTRIBUTION_ID" --query 'Distribution.Status' --output text 2>/dev/null || echo "UNKNOWN")
    DOMAIN=$(aws cloudfront get-distribution --id "$DISTRIBUTION_ID" --query 'Distribution.DomainName' --output text 2>/dev/null || echo "UNKNOWN")

    if [ "$DIST_STATUS" = "Deployed" ]; then
        pass "CloudFront distribution: $DISTRIBUTION_ID (Deployed)"
        echo "  Domain: $DOMAIN"
    else
        warn "CloudFront distribution: $DISTRIBUTION_ID (Status: $DIST_STATUS)"
    fi
else
    info "CloudFront distribution not found (optional)"
fi
echo ""

# Summary
echo "========================================="
echo -e "${GREEN}Infrastructure Verification Complete${NC}"
echo "========================================="
echo ""
echo "Next steps:"
echo "  1. Run end-to-end tests: ./scripts/test-game-lambda.sh"
echo "  2. Check CloudWatch logs for any errors"
echo "  3. Verify custom domain (if configured)"
echo ""
