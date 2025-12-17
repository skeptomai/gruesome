#!/bin/bash
set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

TABLE_NAME="${TABLE_NAME:-gruesome-platform}"

echo "========================================="
echo "Game Metadata Seeding Script"
echo "========================================="
echo ""
echo "Table: $TABLE_NAME"
echo ""

# Function to add game metadata
add_game() {
    local game_id=$1
    local title=$2
    local author=$3
    local description=$4
    local version=$5
    local s3_key=$6

    # Get file size from S3
    local file_size=$(aws s3api head-object --bucket gruesome-games --key "$s3_key" --query ContentLength --output text 2>/dev/null || echo "0")
    local now=$(date +%s)

    echo -e "${YELLOW}→ Adding: $title${NC}"

    aws dynamodb put-item \
        --table-name "$TABLE_NAME" \
        --item "{
            \"PK\": {\"S\": \"GAME#$game_id\"},
            \"SK\": {\"S\": \"METADATA\"},
            \"entity_type\": {\"S\": \"GAME\"},
            \"game_id\": {\"S\": \"$game_id\"},
            \"title\": {\"S\": \"$title\"},
            \"author\": {\"S\": \"$author\"},
            \"description\": {\"S\": \"$description\"},
            \"version\": {\"N\": \"$version\"},
            \"file_size\": {\"N\": \"$file_size\"},
            \"s3_key\": {\"S\": \"$s3_key\"},
            \"created_at\": {\"N\": \"$now\"}
        }" \
        --no-cli-pager > /dev/null

    echo -e "${GREEN}✓ Added: $title (${file_size} bytes)${NC}"
}

# Seed mini-zork
add_game \
    "mini-zork" \
    "DORK I: The Last Great Empire" \
    "Grue Games" \
    "A miniature test adventure game for the Gruesome Z-Machine platform. Not affiliated with Zork or Infocom." \
    "3" \
    "mini-zork.z3"

# Add more games here as needed
# add_game \
#     "zork1" \
#     "Zork I: The Great Underground Empire" \
#     "Infocom" \
#     "The first in the Zork trilogy. Your mission is to find the legendary Treasures of Zork and install yourself as Master of the Great Underground Empire." \
#     "3" \
#     "zork1.z3"

echo ""
echo "========================================="
echo -e "${GREEN}Seeding Complete!${NC}"
echo "========================================="
echo ""
echo "Verify with:"
echo "  aws dynamodb scan --table-name $TABLE_NAME --filter-expression \"begins_with(PK, :pk)\" --expression-attribute-values '{\":pk\":{\"S\":\"GAME#\"}}'"
echo ""
