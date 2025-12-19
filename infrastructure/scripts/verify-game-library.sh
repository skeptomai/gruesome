#!/bin/bash
#
# Gruesome Game Library Verification Script
# Verifies consistency between S3 files and DynamoDB metadata
#
# Usage: ./verify-game-library.sh [staging|prod]
#

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Environment selection
ENV=${1:-prod}
if [ "$ENV" = "staging" ]; then
    S3_BUCKET="gruesome-games-staging"
    DYNAMODB_TABLE="gruesome-platform-staging"
    echo "Verifying STAGING environment..."
elif [ "$ENV" = "prod" ]; then
    S3_BUCKET="gruesome-games"
    DYNAMODB_TABLE="gruesome-platform"
    echo "Verifying PRODUCTION environment..."
else
    echo "Error: Environment must be 'staging' or 'prod'"
    echo "Usage: $0 [staging|prod]"
    exit 1
fi

echo "================================================"
echo "Game Library Verification"
echo "================================================"
echo "S3 Bucket: $S3_BUCKET"
echo "DynamoDB Table: $DYNAMODB_TABLE"
echo ""

ERRORS=0
WARNINGS=0

# Check 1: Verify all DynamoDB entries have corresponding S3 files
echo "Check 1: DynamoDB → S3 Consistency"
echo "-----------------------------------"

GAMES=$(aws dynamodb scan \
    --table-name "$DYNAMODB_TABLE" \
    --filter-expression "entity_type = :game" \
    --expression-attribute-values '{":game":{"S":"GAME"}}' \
    --projection-expression "game_id, s3_key, file_size" \
    --output json)

echo "$GAMES" | jq -r '.Items[] | "\(.game_id.S)|\(.s3_key.S)|\(.file_size.N)"' | while IFS='|' read -r game_id s3_key file_size; do
    # Check s3_key format
    if [[ ! "$s3_key" =~ ^games/.+\.z[0-9]$ ]]; then
        echo -e "${RED}✗${NC} $game_id: Invalid s3_key format: '$s3_key' (should be 'games/{game_id}.z3')"
        ((ERRORS++)) || true
    fi

    # Check S3 file exists
    FULL_S3_PATH="s3://$S3_BUCKET/$s3_key"
    if aws s3 ls "$FULL_S3_PATH" >/dev/null 2>&1; then
        # Get actual file size from S3
        ACTUAL_SIZE=$(aws s3 ls "$FULL_S3_PATH" | awk '{print $3}')

        if [ "$ACTUAL_SIZE" = "$file_size" ]; then
            echo -e "${GREEN}✓${NC} $game_id: S3 file exists with correct size ($file_size bytes)"
        else
            echo -e "${YELLOW}⚠${NC} $game_id: Size mismatch - DynamoDB: $file_size, S3: $ACTUAL_SIZE"
            ((WARNINGS++)) || true
        fi
    else
        echo -e "${RED}✗${NC} $game_id: S3 file MISSING at $FULL_S3_PATH"
        ((ERRORS++)) || true
    fi
done

echo ""

# Check 2: Verify all S3 files have corresponding DynamoDB entries
echo "Check 2: S3 → DynamoDB Consistency"
echo "-----------------------------------"

aws s3 ls "s3://$S3_BUCKET/games/" | awk '{print $4}' | while read -r filename; do
    # Skip empty lines
    [ -z "$filename" ] && continue

    # Extract game_id from filename (remove .z3 extension)
    game_id="${filename%.z3}"

    # Check if DynamoDB entry exists
    RESULT=$(aws dynamodb get-item \
        --table-name "$DYNAMODB_TABLE" \
        --key "{\"PK\": {\"S\": \"GAME#$game_id\"}, \"SK\": {\"S\": \"METADATA\"}}" \
        --output json 2>/dev/null || echo "{}")

    if echo "$RESULT" | jq -e '.Item' >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} $filename: DynamoDB entry exists"
    else
        echo -e "${RED}✗${NC} $filename: DynamoDB entry MISSING for game_id '$game_id'"
        ((ERRORS++)) || true
    fi
done

echo ""

# Check 3: Verify s3_key consistency across all games
echo "Check 3: S3 Key Format Consistency"
echo "-----------------------------------"

INCONSISTENT=$(echo "$GAMES" | jq -r '.Items[] | select(.s3_key.S | startswith("games/") | not) | .game_id.S')

if [ -z "$INCONSISTENT" ]; then
    echo -e "${GREEN}✓${NC} All s3_key values use consistent 'games/' prefix"
else
    echo -e "${RED}✗${NC} Inconsistent s3_key formats found:"
    echo "$INCONSISTENT" | while read -r game_id; do
        echo "  - $game_id"
        ((ERRORS++)) || true
    done
fi

echo ""
echo "================================================"
echo "Verification Summary"
echo "================================================"

if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed!${NC}"
    exit 0
elif [ $ERRORS -eq 0 ]; then
    echo -e "${YELLOW}⚠ $WARNINGS warning(s) found${NC}"
    exit 0
else
    echo -e "${RED}✗ $ERRORS error(s) found${NC}"
    [ $WARNINGS -gt 0 ] && echo -e "${YELLOW}⚠ $WARNINGS warning(s) found${NC}"
    exit 1
fi
