#!/bin/bash
#
# Gruesome Game Upload Script
# Standardized process for uploading games to S3 and DynamoDB
#
# Usage: ./upload-game.sh <game_file> <game_id> <title> <author> <description> [staging|prod]
#
# Example:
#   ./upload-game.sh zork1.z3 zork1 "Zork I: The Great Underground Empire" "Marc Blank & Dave Lebling" "The original classic adventure game" prod
#

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check arguments
if [ $# -lt 5 ]; then
    echo "Usage: $0 <game_file> <game_id> <title> <author> <description> [staging|prod]"
    echo ""
    echo "Arguments:"
    echo "  game_file    - Path to .z3/.z4/.z5 game file"
    echo "  game_id      - Unique game identifier (e.g., 'zork1', 'hhgg')"
    echo "  title        - Full game title (quoted)"
    echo "  author       - Author name(s) (quoted)"
    echo "  description  - Brief description (quoted)"
    echo "  environment  - 'staging' or 'prod' (default: staging)"
    echo ""
    echo "Example:"
    echo "  $0 zork1.z3 zork1 \"Zork I\" \"Marc Blank\" \"Classic adventure\" prod"
    exit 1
fi

GAME_FILE="$1"
GAME_ID="$2"
TITLE="$3"
AUTHOR="$4"
DESCRIPTION="$5"
ENV="${6:-staging}"

# Validate environment
if [ "$ENV" != "staging" ] && [ "$ENV" != "prod" ]; then
    echo -e "${RED}Error: Environment must be 'staging' or 'prod'${NC}"
    exit 1
fi

# Set environment-specific variables
if [ "$ENV" = "staging" ]; then
    S3_BUCKET="gruesome-games-staging"
    DYNAMODB_TABLE="gruesome-platform-staging"
else
    S3_BUCKET="gruesome-games"
    DYNAMODB_TABLE="gruesome-platform"
fi

echo "================================================"
echo "Gruesome Game Upload"
echo "================================================"
echo -e "${BLUE}Environment:${NC} $ENV"
echo -e "${BLUE}Game ID:${NC} $GAME_ID"
echo -e "${BLUE}Title:${NC} $TITLE"
echo -e "${BLUE}Author:${NC} $AUTHOR"
echo ""

# Validate game file exists
if [ ! -f "$GAME_FILE" ]; then
    echo -e "${RED}Error: Game file not found: $GAME_FILE${NC}"
    exit 1
fi

# Extract Z-Machine version from first byte
VERSION=$(xxd -l 1 -p "$GAME_FILE")
VERSION=$((16#$VERSION))

if [ $VERSION -lt 1 ] || [ $VERSION -gt 8 ]; then
    echo -e "${RED}Error: Invalid Z-Machine version: $VERSION${NC}"
    exit 1
fi

# Get file size
FILE_SIZE=$(stat -f%z "$GAME_FILE" 2>/dev/null || stat -c%s "$GAME_FILE" 2>/dev/null)

# Determine file extension based on version
FILE_EXT=".z${VERSION}"
S3_KEY="games/${GAME_ID}${FILE_EXT}"
S3_PATH="s3://${S3_BUCKET}/${S3_KEY}"

echo -e "${BLUE}Z-Machine Version:${NC} $VERSION"
echo -e "${BLUE}File Size:${NC} $FILE_SIZE bytes"
echo -e "${BLUE}S3 Path:${NC} $S3_PATH"
echo ""

# Confirm upload
read -p "Proceed with upload to $ENV? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Upload cancelled"
    exit 0
fi

echo ""
echo "Step 1: Uploading to S3..."
echo "-----------------------------------"

if aws s3 cp "$GAME_FILE" "$S3_PATH"; then
    echo -e "${GREEN}✓ S3 upload successful${NC}"
else
    echo -e "${RED}✗ S3 upload failed${NC}"
    exit 1
fi

echo ""
echo "Step 2: Creating DynamoDB metadata..."
echo "-----------------------------------"

TIMESTAMP=$(date +%s)

ITEM_JSON=$(cat <<EOF
{
  "PK": {"S": "GAME#${GAME_ID}"},
  "SK": {"S": "METADATA"},
  "entity_type": {"S": "GAME"},
  "game_id": {"S": "${GAME_ID}"},
  "title": {"S": "${TITLE}"},
  "author": {"S": "${AUTHOR}"},
  "description": {"S": "${DESCRIPTION}"},
  "version": {"N": "${VERSION}"},
  "file_size": {"N": "${FILE_SIZE}"},
  "s3_key": {"S": "${S3_KEY}"},
  "created_at": {"N": "${TIMESTAMP}"}
}
EOF
)

if aws dynamodb put-item --table-name "$DYNAMODB_TABLE" --item "$ITEM_JSON"; then
    echo -e "${GREEN}✓ DynamoDB metadata created${NC}"
else
    echo -e "${RED}✗ DynamoDB metadata creation failed${NC}"
    echo -e "${YELLOW}Warning: S3 file uploaded but metadata failed. Consider manual cleanup.${NC}"
    exit 1
fi

echo ""
echo "Step 3: Verification..."
echo "-----------------------------------"

# Verify S3 file
if aws s3 ls "$S3_PATH" >/dev/null 2>&1; then
    echo -e "${GREEN}✓ S3 file verified${NC}"
else
    echo -e "${RED}✗ S3 file verification failed${NC}"
    exit 1
fi

# Verify DynamoDB entry
RESULT=$(aws dynamodb get-item \
    --table-name "$DYNAMODB_TABLE" \
    --key "{\"PK\": {\"S\": \"GAME#${GAME_ID}\"}, \"SK\": {\"S\": \"METADATA\"}}" \
    --output json)

if echo "$RESULT" | jq -e '.Item' >/dev/null 2>&1; then
    echo -e "${GREEN}✓ DynamoDB entry verified${NC}"

    # Verify metadata fields
    STORED_TITLE=$(echo "$RESULT" | jq -r '.Item.title.S')
    STORED_S3_KEY=$(echo "$RESULT" | jq -r '.Item.s3_key.S')

    echo ""
    echo "Stored metadata:"
    echo "  Title: $STORED_TITLE"
    echo "  S3 Key: $STORED_S3_KEY"
else
    echo -e "${RED}✗ DynamoDB entry verification failed${NC}"
    exit 1
fi

echo ""
echo "================================================"
echo -e "${GREEN}✓ Upload Complete!${NC}"
echo "================================================"
echo ""
echo "Game '$TITLE' successfully uploaded to $ENV"
echo "Game ID: $GAME_ID"
echo "S3: $S3_PATH"
echo "DynamoDB: $DYNAMODB_TABLE"
echo ""
echo "Next steps:"
echo "1. Test the game loads correctly in staging"
echo "2. Run verification: ./verify-game-library.sh $ENV"
if [ "$ENV" = "staging" ]; then
    echo "3. If verified, upload to prod:"
    echo "   $0 \"$GAME_FILE\" \"$GAME_ID\" \"$TITLE\" \"$AUTHOR\" \"$DESCRIPTION\" prod"
fi
