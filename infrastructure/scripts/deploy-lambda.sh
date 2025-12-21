#!/bin/bash
# Deploy Lambda function code (Rust)
# Usage: ./deploy-lambda.sh [function] [environment]
#   function: auth|game|admin
#   environment: staging|prod

set -e

FUNCTION="${1}"
ENV="${2:-staging}"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

error() {
    echo -e "${RED}Error: $1${NC}" >&2
    exit 1
}

info() {
    echo -e "${YELLOW}→ $1${NC}"
}

success() {
    echo -e "${GREEN}✓ $1${NC}"
}

if [ -z "$FUNCTION" ]; then
    error "Function name required\nUsage: $0 [auth|game|admin] [staging|prod]"
fi

# Validate function
if [[ ! "$FUNCTION" =~ ^(auth|game|admin)$ ]]; then
    error "Function must be auth, game, or admin"
fi

# Validate environment
if [[ ! "$ENV" =~ ^(staging|prod)$ ]]; then
    error "Environment must be staging or prod"
fi

echo "================================================"
echo "Deploying Lambda: $FUNCTION ($ENV)"
echo "================================================"
echo ""

# Navigate to lambda directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/../lambda/gruesome-api"

# Clean previous build for this function to ensure fresh build
info "Cleaning previous build artifacts..."
rm -rf "target/lambda/$FUNCTION"
success "Cleaned target/lambda/$FUNCTION"

# Build Lambda function
info "Building Lambda function..."
BUILD_OUTPUT=$(cargo lambda build --release --arm64 --bin "$FUNCTION" 2>&1)
BUILD_TIME=$(echo "$BUILD_OUTPUT" | grep -o "Finished.*in [0-9.]*s" | grep -o "[0-9.]*s" | tr -d 's')

echo "$BUILD_OUTPUT"

# Check if build succeeded
if [ ! -f "target/lambda/$FUNCTION/bootstrap" ]; then
    error "Bootstrap binary not created. Build failed."
fi

# Verify build was not cached (should take >10 seconds)
if (( $(echo "$BUILD_TIME < 5.0" | bc -l) )); then
    echo -e "${YELLOW}⚠ Warning: Build finished very quickly ($BUILD_TIME s)${NC}"
    echo -e "${YELLOW}  This might indicate a cached build.${NC}"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        error "Deployment cancelled"
    fi
fi

# Get bootstrap binary info
BOOTSTRAP_PATH="target/lambda/$FUNCTION/bootstrap"
BOOTSTRAP_SIZE=$(ls -lh "$BOOTSTRAP_PATH" | awk '{print $5}')
BOOTSTRAP_DATE=$(ls -l "$BOOTSTRAP_PATH" | awk '{print $6, $7, $8}')
success "Bootstrap binary created: $BOOTSTRAP_SIZE (modified: $BOOTSTRAP_DATE)"

# Create zip file
info "Creating bootstrap.zip..."
cd "target/lambda/$FUNCTION"

# Remove old zip if exists
rm -f bootstrap.zip

# Create new zip
zip -q bootstrap.zip bootstrap

# Verify zip was created
if [ ! -f "bootstrap.zip" ]; then
    error "Failed to create bootstrap.zip"
fi

ZIP_SIZE=$(ls -lh bootstrap.zip | awk '{print $5}')
ZIP_DATE=$(ls -l bootstrap.zip | awk '{print $6, $7, $8}')
success "bootstrap.zip created: $ZIP_SIZE (modified: $ZIP_DATE)"

# Get Lambda function name based on environment and function
info "Looking up Lambda function name..."

if [ "$ENV" == "staging" ]; then
    case "$FUNCTION" in
        auth)
            # Query CloudFormation for actual name
            LAMBDA_NAME=$(aws cloudformation describe-stack-resources \
                --stack-name GruesomeBackendStackStaging \
                --query 'StackResources[?LogicalResourceId==`AuthFunction`].PhysicalResourceId' \
                --output text)
            ;;
        game)
            LAMBDA_NAME=$(aws cloudformation describe-stack-resources \
                --stack-name GruesomeBackendStackStaging \
                --query 'StackResources[?LogicalResourceId==`GameFunction`].PhysicalResourceId' \
                --output text)
            ;;
        admin)
            # Admin is in separate stack
            LAMBDA_NAME=$(aws lambda list-functions \
                --query 'Functions[?contains(FunctionName, `AdminStaging`)].FunctionName' \
                --output text | head -1)
            ;;
    esac
else
    case "$FUNCTION" in
        auth)
            LAMBDA_NAME=$(aws cloudformation describe-stack-resources \
                --stack-name GruesomeBackendStack \
                --query 'StackResources[?LogicalResourceId==`AuthFunction`].PhysicalResourceId' \
                --output text)
            ;;
        game)
            LAMBDA_NAME=$(aws cloudformation describe-stack-resources \
                --stack-name GruesomeBackendStack \
                --query 'StackResources[?LogicalResourceId==`GameFunction`].PhysicalResourceId' \
                --output text)
            ;;
        admin)
            # Admin is in separate stack
            LAMBDA_NAME=$(aws lambda list-functions \
                --query 'Functions[?contains(FunctionName, `Admin`) && !contains(FunctionName, `Staging`)].FunctionName' \
                --output text | head -1)
            ;;
    esac
fi

if [ -z "$LAMBDA_NAME" ] || [ "$LAMBDA_NAME" == "None" ]; then
    error "Could not find Lambda function for $FUNCTION in $ENV environment"
fi

success "Found Lambda function: $LAMBDA_NAME"

# Get full path to bootstrap.zip
ZIP_PATH="$(pwd)/bootstrap.zip"

# Deploy to Lambda
info "Deploying to AWS Lambda..."
DEPLOY_OUTPUT=$(aws lambda update-function-code \
    --function-name "$LAMBDA_NAME" \
    --zip-file "fileb://$ZIP_PATH" \
    --query '{FunctionName:FunctionName,LastModified:LastModified,CodeSize:CodeSize}' \
    --output json)

echo "$DEPLOY_OUTPUT" | jq '.'

DEPLOY_SIZE=$(echo "$DEPLOY_OUTPUT" | jq -r '.CodeSize')
success "Deployed $DEPLOY_SIZE bytes to $LAMBDA_NAME"

# Wait for function to be ready
info "Waiting for Lambda function to be ready..."
sleep 3

# Test the deployment
info "Testing deployment..."

if [ "$ENV" == "staging" ]; then
    API_BASE="https://api-staging.gruesome.skeptomai.com"
else
    API_BASE="https://api.gruesome.skeptomai.com"
fi

# Test health endpoint
HEALTH_RESPONSE=$(curl -s "$API_BASE/health" || echo "{}")
HEALTH_STATUS=$(echo "$HEALTH_RESPONSE" | jq -r '.status' 2>/dev/null || echo "unknown")

if [ "$HEALTH_STATUS" == "healthy" ]; then
    success "Health check passed"
else
    echo -e "${YELLOW}⚠ Warning: Health check returned: $HEALTH_STATUS${NC}"
fi

# For game function, test the games endpoint
if [ "$FUNCTION" == "game" ]; then
    GAMES_RESPONSE=$(curl -s "$API_BASE/api/games" || echo "{}")
    GAMES_COUNT=$(echo "$GAMES_RESPONSE" | jq -r '.games | length' 2>/dev/null || echo "0")

    if [ "$GAMES_COUNT" -gt 0 ]; then
        success "Games endpoint returned $GAMES_COUNT games"
    else
        echo -e "${YELLOW}⚠ Warning: Games endpoint returned unexpected response${NC}"
        echo "$GAMES_RESPONSE" | jq '.' 2>/dev/null || echo "$GAMES_RESPONSE"
    fi
fi

echo ""
echo "================================================"
echo -e "${GREEN}Deployment Complete!${NC}"
echo "================================================"
echo "Function: $FUNCTION"
echo "Environment: $ENV"
echo "Lambda: $LAMBDA_NAME"
echo "API Base: $API_BASE"
echo ""
echo "Next steps:"
echo "  1. Test the API endpoints manually"
echo "  2. Check Lambda logs: aws logs tail /aws/lambda/$LAMBDA_NAME --since 5m"
echo "  3. Monitor for errors in production"
echo ""
