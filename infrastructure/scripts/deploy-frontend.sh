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

# Production deployment safety check
if [ "$ENV" == "prod" ]; then
    echo ""
    echo "⚠️  WARNING: You are about to deploy to PRODUCTION ⚠️"
    echo ""
    echo "This will update the live site at gruesome.skeptomai.com"
    echo ""
    read -p "Type 'DEPLOY TO PRODUCTION' to continue: " confirm
    if [ "$confirm" != "DEPLOY TO PRODUCTION" ]; then
        echo "Production deployment cancelled."
        exit 1
    fi
    echo ""
fi

# Configuration
if [ "$ENV" == "staging" ]; then
    BUCKET="gruesome-frontend-staging"
    DISTRIBUTION_ID="E1M8DHMS3GCUDX"
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

# Generate build version info (git commit hash + timestamp)
COMMIT_HASH=$(git -C .. rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_TIME=$(date -u +"%Y-%m-%d %H:%M UTC")
BUILD_VERSION="${COMMIT_HASH} @ ${BUILD_TIME}"

echo "Build version: $BUILD_VERSION"
echo ""

# Inject build version into HTML
if [ "$ENV" == "staging" ]; then
    # Staging: Show commit hash + timestamp (visible watermark)
    sed -e "s/<span id=\"build-version\">DEV<\/span>/<span id=\"build-version\">$BUILD_VERSION<\/span>/" \
        -e 's/style="position: fixed; bottom: 0; right: 0; background: rgba(0,0,0,0.7); color: #666; padding: 5px 10px; font-size: 10px; font-family: monospace; z-index: 9999; display: none;"/style="position: fixed; bottom: 0; right: 0; background: rgba(0,0,0,0.7); color: #666; padding: 5px 10px; font-size: 10px; font-family: monospace; z-index: 9999; display: block;"/' \
        index.html > index.deploy.html
else
    # Production: Show release version from git tag (visible watermark)
    RELEASE_VERSION=$(git -C .. describe --tags --abbrev=0 2>/dev/null || echo "unknown")
    sed -e "s/<span id=\"build-version\">DEV<\/span>/<span id=\"build-version\">$RELEASE_VERSION<\/span>/" \
        -e 's/style="position: fixed; bottom: 0; right: 0; background: rgba(0,0,0,0.7); color: #666; padding: 5px 10px; font-size: 10px; font-family: monospace; z-index: 9999; display: none;"/style="position: fixed; bottom: 0; right: 0; background: rgba(0,0,0,0.7); color: #666; padding: 5px 10px; font-size: 10px; font-family: monospace; z-index: 9999; display: block;"/' \
        index.html > index.deploy.html
fi

# Upload files to S3
echo "Uploading files to S3..."
aws s3 cp index.deploy.html "s3://$BUCKET/index.html" \
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

# Cleanup temporary file
rm -f index.deploy.html
