#!/bin/bash
# Infrastructure Verification Script
# Tests all deployed AWS resources for the Gruesome platform
# Created: 2025-12-16

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
FRONTEND_DOMAIN="gruesome.skeptomai.com"
API_DOMAIN="api.gruesome.skeptomai.com"
API_GATEWAY_ID="viq8oiws3m"
REGION="us-west-1"
CERT_REGION="us-east-1"
TABLE_NAME="gruesome-platform"
SAVES_BUCKET="gruesome-saves"
USER_POOL_ID="us-west-1_zSPJeB5x0"
FRONTEND_DISTRIBUTION_ID="E36HKKVL2VZOZD"
API_DISTRIBUTION_ID="E2GRMKUTDD19Z6"

# Counters
PASSED=0
FAILED=0
WARNINGS=0

# Helper functions
print_header() {
    echo -e "\n${BLUE}============================================${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}============================================${NC}\n"
}

print_test() {
    echo -e "${YELLOW}[TEST]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[✓]${NC} $1"
    PASSED=$((PASSED + 1))
}

print_failure() {
    echo -e "${RED}[✗]${NC} $1"
    FAILED=$((FAILED + 1))
}

print_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
    WARNINGS=$((WARNINGS + 1))
}

print_info() {
    echo -e "${BLUE}[ℹ]${NC} $1"
}

# Test functions

test_dns_resolution() {
    print_header "DNS Resolution Tests"

    print_test "Checking frontend DNS ($FRONTEND_DOMAIN)..."
    if dig +short $FRONTEND_DOMAIN A | grep -q "."; then
        IPS=$(dig +short $FRONTEND_DOMAIN A | tr '\n' ', ' | sed 's/,$//')
        print_success "Frontend DNS resolves: $IPS"
    else
        print_failure "Frontend DNS does not resolve"
    fi

    print_test "Checking API DNS ($API_DOMAIN)..."
    if dig +short $API_DOMAIN A | grep -q "."; then
        IPS=$(dig +short $API_DOMAIN A | tr '\n' ', ' | sed 's/,$//')
        print_success "API DNS resolves: $IPS"
    else
        print_failure "API DNS does not resolve"
    fi
}

test_ssl_certificates() {
    print_header "SSL Certificate Tests"

    print_test "Checking frontend SSL certificate..."
    CERT_SUBJECT=$(echo | openssl s_client -connect $FRONTEND_DOMAIN:443 -servername $FRONTEND_DOMAIN 2>/dev/null | openssl x509 -noout -subject 2>/dev/null)
    if echo "$CERT_SUBJECT" | grep -q "gruesome.skeptomai.com"; then
        print_success "Frontend SSL certificate valid: $CERT_SUBJECT"
    else
        print_failure "Frontend SSL certificate invalid or missing"
    fi

    print_test "Checking API SSL certificate..."
    CERT_SUBJECT=$(echo | openssl s_client -connect $API_DOMAIN:443 -servername $API_DOMAIN 2>/dev/null | openssl x509 -noout -subject 2>/dev/null)
    if echo "$CERT_SUBJECT" | grep -q "gruesome.skeptomai.com"; then
        print_success "API SSL certificate valid: $CERT_SUBJECT"
    else
        print_failure "API SSL certificate invalid or missing"
    fi

    print_test "Checking certificate expiration..."
    EXPIRY=$(echo | openssl s_client -connect $API_DOMAIN:443 -servername $API_DOMAIN 2>/dev/null | openssl x509 -noout -enddate 2>/dev/null | cut -d= -f2)
    print_info "Certificate expires: $EXPIRY"
}

test_api_gateway() {
    print_header "API Gateway Tests"

    print_test "Testing health check endpoint..."
    RESPONSE=$(curl -s "https://${API_DOMAIN}/health" 2>/dev/null)
    if echo "$RESPONSE" | grep -q '"status":"healthy"'; then
        print_success "Health check working"
    else
        print_failure "Health check failed: $RESPONSE"
    fi

    print_test "Testing CloudFront cache policy..."
    HEADERS=$(curl -s -I "https://${API_DOMAIN}/health" 2>/dev/null)
    if echo "$HEADERS" | grep -qi "x-cache"; then
        CACHE_STATUS=$(echo "$HEADERS" | grep -i "x-cache" | cut -d: -f2 | tr -d '[:space:]')
        print_info "CloudFront cache status: $CACHE_STATUS"
    else
        print_warning "CloudFront headers not found (may be first request)"
    fi
}

test_lambda_function() {
    print_header "Lambda Function Tests"

    print_test "Checking Lambda function exists..."
    FUNCTIONS=$(aws lambda list-functions --region $REGION --query "Functions[?contains(FunctionName, 'AuthFunction')].FunctionName" --output text 2>/dev/null)

    if [ -n "$FUNCTIONS" ]; then
        print_success "Lambda function found: $FUNCTIONS"

        print_test "Checking Lambda configuration..."
        RUNTIME=$(aws lambda get-function-configuration --function-name $FUNCTIONS --region $REGION --query 'Runtime' --output text 2>/dev/null)
        TIMEOUT=$(aws lambda get-function-configuration --function-name $FUNCTIONS --region $REGION --query 'Timeout' --output text 2>/dev/null)
        ARCH=$(aws lambda get-function-configuration --function-name $FUNCTIONS --region $REGION --query 'Architectures[0]' --output text 2>/dev/null)

        print_info "Runtime: $RUNTIME, Timeout: ${TIMEOUT}s, Architecture: $ARCH"
    else
        print_failure "Lambda function not found"
    fi
}

test_dynamodb() {
    print_header "DynamoDB Tests"

    print_test "Checking DynamoDB table exists..."
    if aws dynamodb describe-table --table-name $TABLE_NAME --region $REGION >/dev/null 2>&1; then
        print_success "DynamoDB table exists: $TABLE_NAME"

        print_test "Checking table configuration..."
        STATUS=$(aws dynamodb describe-table --table-name $TABLE_NAME --region $REGION --query 'Table.TableStatus' --output text 2>/dev/null)
        BILLING=$(aws dynamodb describe-table --table-name $TABLE_NAME --region $REGION --query 'Table.BillingModeSummary.BillingMode' --output text 2>/dev/null)
        ITEM_COUNT=$(aws dynamodb describe-table --table-name $TABLE_NAME --region $REGION --query 'Table.ItemCount' --output text 2>/dev/null)

        print_info "Status: $STATUS, Billing: $BILLING, Items: $ITEM_COUNT"

        print_test "Checking Global Secondary Indexes..."
        GSI=$(aws dynamodb describe-table --table-name $TABLE_NAME --region $REGION --query 'Table.GlobalSecondaryIndexes[*].IndexName' --output text 2>/dev/null)
        if [ -n "$GSI" ]; then
            print_success "GSI found: $GSI"
        else
            print_warning "No GSI found"
        fi
    else
        print_failure "DynamoDB table not found"
    fi
}

test_s3_bucket() {
    print_header "S3 Bucket Tests"

    print_test "Checking S3 saves bucket exists..."
    if aws s3 ls s3://$SAVES_BUCKET >/dev/null 2>&1; then
        print_success "S3 bucket exists: $SAVES_BUCKET"

        print_test "Checking bucket versioning..."
        VERSIONING=$(aws s3api get-bucket-versioning --bucket $SAVES_BUCKET --query 'Status' --output text 2>/dev/null)
        if [ "$VERSIONING" = "Enabled" ]; then
            print_success "Bucket versioning enabled"
        else
            print_warning "Bucket versioning not enabled"
        fi

        print_test "Checking CORS configuration..."
        if aws s3api get-bucket-cors --bucket $SAVES_BUCKET >/dev/null 2>&1; then
            print_success "CORS configuration present"
        else
            print_warning "CORS configuration not found"
        fi
    else
        print_failure "S3 bucket not found"
    fi

    print_test "Checking frontend bucket..."
    if aws s3 ls s3://gruesome-frontend >/dev/null 2>&1; then
        print_success "Frontend bucket exists: gruesome-frontend"
        OBJECT_COUNT=$(aws s3 ls s3://gruesome-frontend --recursive 2>/dev/null | wc -l)
        if [ "$OBJECT_COUNT" -gt 0 ]; then
            print_info "Frontend objects: $OBJECT_COUNT"
        else
            print_warning "Frontend bucket is empty (no website deployed)"
        fi
    else
        print_failure "Frontend bucket not found"
    fi
}

test_cognito() {
    print_header "Cognito Tests"

    print_test "Checking Cognito User Pool..."
    if aws cognito-idp describe-user-pool --user-pool-id $USER_POOL_ID --region $REGION >/dev/null 2>&1; then
        print_success "Cognito User Pool exists: $USER_POOL_ID"

        print_test "Checking pool configuration..."
        POOL_NAME=$(aws cognito-idp describe-user-pool --user-pool-id $USER_POOL_ID --region $REGION --query 'UserPool.Name' --output text 2>/dev/null)
        USER_COUNT=$(aws cognito-idp describe-user-pool --user-pool-id $USER_POOL_ID --region $REGION --query 'UserPool.EstimatedNumberOfUsers' --output text 2>/dev/null)

        print_info "Pool name: $POOL_NAME, Users: $USER_COUNT"

        print_test "Checking User Pool clients..."
        CLIENT_COUNT=$(aws cognito-idp list-user-pool-clients --user-pool-id $USER_POOL_ID --region $REGION --query 'UserPoolClients | length(@)' --output text 2>/dev/null)
        if [ "$CLIENT_COUNT" -gt 0 ]; then
            print_success "User Pool clients: $CLIENT_COUNT"
        else
            print_warning "No User Pool clients found"
        fi
    else
        print_failure "Cognito User Pool not found"
    fi
}

test_cloudfront() {
    print_header "CloudFront Distribution Tests"

    print_test "Checking frontend CloudFront distribution..."
    STATUS=$(aws cloudfront get-distribution --id $FRONTEND_DISTRIBUTION_ID --query 'Distribution.Status' --output text 2>/dev/null)
    if [ "$STATUS" = "Deployed" ]; then
        print_success "Frontend distribution deployed: $FRONTEND_DISTRIBUTION_ID"
    else
        print_warning "Frontend distribution status: $STATUS"
    fi

    print_test "Checking API CloudFront distribution..."
    STATUS=$(aws cloudfront get-distribution --id $API_DISTRIBUTION_ID --query 'Distribution.Status' --output text 2>/dev/null)
    if [ "$STATUS" = "Deployed" ]; then
        print_success "API distribution deployed: $API_DISTRIBUTION_ID"

        print_test "Checking API distribution origin..."
        ORIGIN=$(aws cloudfront get-distribution --id $API_DISTRIBUTION_ID --query 'Distribution.DistributionConfig.Origins.Items[0].DomainName' --output text 2>/dev/null)
        if echo "$ORIGIN" | grep -q "execute-api"; then
            print_success "API origin configured correctly: $ORIGIN"
        else
            print_warning "API origin unexpected: $ORIGIN"
        fi
    else
        print_warning "API distribution status: $STATUS"
    fi
}

test_cloudformation_stacks() {
    print_header "CloudFormation Stack Tests"

    STACKS=(
        "GruesomeDnsStack:$CERT_REGION"
        "GruesomeDataStack:$REGION"
        "GruesomeAuthStack:$REGION"
        "GruesomeBackendStack:$REGION"
        "GruesomeFrontendStack:$CERT_REGION"
    )

    for stack_info in "${STACKS[@]}"; do
        STACK_NAME=$(echo $stack_info | cut -d: -f1)
        STACK_REGION=$(echo $stack_info | cut -d: -f2)

        print_test "Checking $STACK_NAME in $STACK_REGION..."
        STATUS=$(aws cloudformation describe-stacks --stack-name $STACK_NAME --region $STACK_REGION --query 'Stacks[0].StackStatus' --output text 2>/dev/null)

        if [ "$STATUS" = "CREATE_COMPLETE" ] || [ "$STATUS" = "UPDATE_COMPLETE" ]; then
            print_success "$STACK_NAME: $STATUS"
        else
            print_failure "$STACK_NAME: $STATUS"
        fi
    done
}

test_iam_roles() {
    print_header "IAM Role Tests"

    print_test "Checking Lambda execution role..."
    ROLES=$(aws iam list-roles --query "Roles[?contains(RoleName, 'AuthFunction')].RoleName" --output text 2>/dev/null)

    if [ -n "$ROLES" ]; then
        print_success "Lambda execution role found: $ROLES"

        print_test "Checking attached policies..."
        POLICIES=$(aws iam list-attached-role-policies --role-name $ROLES --query 'AttachedPolicies[*].PolicyName' --output text 2>/dev/null)
        if echo "$POLICIES" | grep -q "AWSLambdaBasicExecutionRole"; then
            print_success "Basic execution policy attached"
        else
            print_warning "Basic execution policy not found"
        fi
    else
        print_failure "Lambda execution role not found"
    fi
}

test_authentication() {
    print_header "Authentication Endpoint Tests"

    # Generate random test user
    TEST_USER="testuser_$(date +%s)"
    TEST_EMAIL="${TEST_USER}@example.com"
    TEST_PASSWORD="TestPass123"

    print_test "Testing user signup..."
    SIGNUP_RESPONSE=$(curl -s -X POST "https://${API_DOMAIN}/api/auth/signup" \
        -H "Content-Type: application/json" \
        -d "{\"email\":\"${TEST_EMAIL}\",\"password\":\"${TEST_PASSWORD}\",\"username\":\"${TEST_USER}\"}" 2>/dev/null)

    if echo "$SIGNUP_RESPONSE" | grep -q '"user_id"'; then
        print_success "User signup successful"
        USER_ID=$(echo "$SIGNUP_RESPONSE" | jq -r '.user_id' 2>/dev/null)
        print_info "Created user: $TEST_USER (ID: ${USER_ID:0:20}...)"
    else
        print_failure "User signup failed: $SIGNUP_RESPONSE"
        return
    fi

    print_test "Testing user login..."
    LOGIN_RESPONSE=$(curl -s -X POST "https://${API_DOMAIN}/api/auth/login" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"${TEST_USER}\",\"password\":\"${TEST_PASSWORD}\"}" 2>/dev/null)

    if echo "$LOGIN_RESPONSE" | grep -q '"access_token"'; then
        print_success "User login successful"
        ACCESS_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.access_token' 2>/dev/null)
        REFRESH_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.refresh_token' 2>/dev/null)
        EXPIRES_IN=$(echo "$LOGIN_RESPONSE" | jq -r '.expires_in' 2>/dev/null)
        print_info "Access token received (expires in ${EXPIRES_IN}s)"
    else
        print_failure "User login failed: $LOGIN_RESPONSE"
        return
    fi

    print_test "Testing get profile endpoint..."
    PROFILE_RESPONSE=$(curl -s -X GET "https://${API_DOMAIN}/api/auth/me" \
        -H "Authorization: Bearer ${ACCESS_TOKEN}" 2>/dev/null)

    if echo "$PROFILE_RESPONSE" | grep -q '"profile"'; then
        print_success "Get profile successful"
        PROFILE_EMAIL=$(echo "$PROFILE_RESPONSE" | jq -r '.profile.email' 2>/dev/null)
        PROFILE_USERNAME=$(echo "$PROFILE_RESPONSE" | jq -r '.profile.username' 2>/dev/null)
        print_info "Profile: $PROFILE_USERNAME <$PROFILE_EMAIL>"
    else
        print_failure "Get profile failed: $PROFILE_RESPONSE"
    fi

    print_test "Testing token refresh..."
    REFRESH_RESPONSE=$(curl -s -X POST "https://${API_DOMAIN}/api/auth/refresh" \
        -H "Content-Type: application/json" \
        -d "{\"refresh_token\":\"${REFRESH_TOKEN}\"}" 2>/dev/null)

    if echo "$REFRESH_RESPONSE" | grep -q '"access_token"'; then
        print_success "Token refresh successful"
        NEW_TOKEN=$(echo "$REFRESH_RESPONSE" | jq -r '.access_token' 2>/dev/null)
        print_info "New access token received"
    else
        print_failure "Token refresh failed: $REFRESH_RESPONSE"
    fi

    print_test "Testing API Gateway direct access..."
    DIRECT_RESPONSE=$(curl -s "https://${API_GATEWAY_ID}.execute-api.${REGION}.amazonaws.com/health" 2>/dev/null)
    if echo "$DIRECT_RESPONSE" | grep -q '"status":"healthy"'; then
        print_success "API Gateway direct access working"
    else
        print_failure "API Gateway direct access failed: $DIRECT_RESPONSE"
    fi
}

test_connectivity() {
    print_header "End-to-End Connectivity Tests"

    print_test "Testing complete request flow (CloudFront → API Gateway → Lambda)..."
    START_TIME=$(date +%s%3N)
    RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}\nTIME_TOTAL:%{time_total}" \
        "https://${API_DOMAIN}/health" 2>/dev/null)
    END_TIME=$(date +%s%3N)

    HTTP_CODE=$(echo "$RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
    TIME_TOTAL=$(echo "$RESPONSE" | grep "TIME_TOTAL" | cut -d: -f2)
    BODY=$(echo "$RESPONSE" | sed -n '1p')

    if [ "$HTTP_CODE" = "200" ] && echo "$BODY" | grep -q "healthy"; then
        print_success "End-to-end connectivity working (${TIME_TOTAL}s)"
    else
        print_failure "End-to-end connectivity failed (HTTP $HTTP_CODE)"
        print_info "Response: $BODY"
    fi
}

# Main execution
print_header "Gruesome Platform Infrastructure Verification"
print_info "Date: $(date)"
print_info "Region: $REGION"
print_info "Certificate Region: $CERT_REGION"
echo ""

# Run all tests
test_cloudformation_stacks
test_dns_resolution
test_ssl_certificates
test_cloudfront
test_api_gateway
test_lambda_function
test_dynamodb
test_s3_bucket
test_cognito
test_iam_roles
test_connectivity
test_authentication

# Summary
print_header "Verification Summary"
echo -e "${GREEN}Passed:   $PASSED${NC}"
echo -e "${RED}Failed:   $FAILED${NC}"
echo -e "${YELLOW}Warnings: $WARNINGS${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All critical tests passed!${NC}"
    echo -e "${GREEN}Infrastructure is operational.${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed.${NC}"
    echo -e "${YELLOW}Review the failures above and check AWS Console for details.${NC}"
    exit 1
fi
