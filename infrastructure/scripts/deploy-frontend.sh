#!/bin/bash
# Deploy frontend to S3 and invalidate CloudFront cache
# Usage: ./deploy-frontend.sh [staging|prod]

set -e

ENV="${1:-prod}"

if [ "$ENV" != "staging" ] && [ "$ENV" != "prod" ]; then
    echo "Error: Environment must be 'staging' or 'prod'"
    echo "Usage: $0 [staging|prod]"
    exit 1
fi

# Configuration
if [ "$ENV" == "staging" ]; then
    BUCKET="gruesome-frontend-staging"
    DISTRIBUTION_ID=$(aws cloudfront list-distributions --query "DistributionList.Items[?Aliases.Items[?contains(@, 'staging.gruesome')]].Id" --output text)
    DOMAIN="staging.gruesome.skeptomai.com"
else
    BUCKET="gruesome-frontend"
    DISTRIBUTION_ID="E36HKKVL2VZOZD"
    DOMAIN="gruesome.skeptomai.com"
fi

echo "================================================"
echo "Deploying Frontend to $ENV"
echo "================================================"
echo "Bucket: $BUCKET"
echo "Distribution: $DISTRIBUTION_ID"
echo "Domain: $DOMAIN"
echo ""

# Navigate to frontend directory
cd "$(dirname "$0")/../../frontend"

# Upload files to S3
echo "Uploading files to S3..."
aws s3 cp index.html "s3://$BUCKET/index.html" \
    --content-type "text/html" \
    --cache-control "no-cache, no-store, must-revalidate" \
    --metadata-directive REPLACE

aws s3 cp style.css "s3://$BUCKET/style.css" \
    --content-type "text/css" \
    --cache-control "no-cache, no-store, must-revalidate" \
    --metadata-directive REPLACE

aws s3 cp app.js "s3://$BUCKET/app.js" \
    --content-type "application/javascript" \
    --cache-control "no-cache, no-store, must-revalidate" \
    --metadata-directive REPLACE

aws s3 cp dev-config.js "s3://$BUCKET/dev-config.js" \
    --content-type "application/javascript" \
    --cache-control "no-cache, no-store, must-revalidate" \
    --metadata-directive REPLACE

# Upload WASM files if they exist
if [ -f "gruesome.js" ]; then
    echo "Uploading WASM files..."
    aws s3 cp gruesome.js "s3://$BUCKET/gruesome.js" \
        --content-type "application/javascript" \
        --cache-control "public, max-age=31536000"

    aws s3 cp gruesome_bg.wasm "s3://$BUCKET/gruesome_bg.wasm" \
        --content-type "application/wasm" \
        --cache-control "public, max-age=31536000"
fi

echo ""
echo "Creating CloudFront invalidation..."
INVALIDATION_ID=$(aws cloudfront create-invalidation \
    --distribution-id "$DISTRIBUTION_ID" \
    --paths "/*" \
    --query 'Invalidation.Id' \
    --output text)

echo "Invalidation created: $INVALIDATION_ID"
echo ""
echo "================================================"
echo "Deployment complete!"
echo "================================================"
echo "URL: https://$DOMAIN"
echo ""
echo "Note: CloudFront invalidation may take 1-3 minutes"
echo "Check status: aws cloudfront get-invalidation --distribution-id $DISTRIBUTION_ID --id $INVALIDATION_ID"
