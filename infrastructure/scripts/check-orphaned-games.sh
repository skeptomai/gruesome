#!/bin/bash
set -e

# Check for orphaned games in DynamoDB and S3
# Usage: ./check-orphaned-games.sh [staging|production] [--delete]

ENVIRONMENT=${1:-staging}
DELETE_MODE=false

if [[ "$2" == "--delete" ]]; then
    DELETE_MODE=true
fi

# Set environment-specific variables
if [[ "$ENVIRONMENT" == "production" ]]; then
    TABLE_NAME="gruesome-platform"
    BUCKET_NAME="gruesome-games"
elif [[ "$ENVIRONMENT" == "staging" ]]; then
    TABLE_NAME="gruesome-platform-staging"
    BUCKET_NAME="gruesome-games-staging"
else
    echo "Error: Environment must be 'staging' or 'production'"
    exit 1
fi

echo "================================================"
echo "Checking Orphaned Games: $ENVIRONMENT"
echo "================================================"
echo "Table: $TABLE_NAME"
echo "Bucket: $BUCKET_NAME"
echo "Delete Mode: $DELETE_MODE"
echo ""

ORPHANS_FOUND=0

# Check 1: DynamoDB metadata without S3 files
echo "=== Check 1: Metadata without S3 files ==="
echo ""

METADATA_ORPHANS=$(mktemp)
aws dynamodb scan \
    --table-name "$TABLE_NAME" \
    --filter-expression "SK = :sk" \
    --expression-attribute-values '{":sk":{"S":"METADATA"}}' \
    --projection-expression "game_id,title,s3_key,PK,SK" \
    --output json > "$METADATA_ORPHANS"

GAME_COUNT=$(jq -r '.Count' "$METADATA_ORPHANS")
echo "Found $GAME_COUNT games in DynamoDB"
echo ""

jq -r '.Items[] | "\(.game_id.S)|\(.title.S)|\(.s3_key.S)|\(.PK.S)|\(.SK.S)"' "$METADATA_ORPHANS" | while IFS='|' read -r game_id title s3_key pk sk; do
    # Check if S3 file exists
    if aws s3 ls "s3://$BUCKET_NAME/$s3_key" >/dev/null 2>&1; then
        echo "✓ $game_id: S3 file exists"
    else
        echo "✗ ORPHAN: $game_id ($title)"
        echo "  Metadata exists but S3 file missing: s3://$BUCKET_NAME/$s3_key"
        ORPHANS_FOUND=$((ORPHANS_FOUND + 1))

        if [[ "$DELETE_MODE" == true ]]; then
            echo "  Deleting metadata..."
            aws dynamodb delete-item \
                --table-name "$TABLE_NAME" \
                --key "{\"PK\":{\"S\":\"$pk\"},\"SK\":{\"S\":\"$sk\"}}"
            echo "  ✓ Deleted metadata for $game_id"
        fi
    fi
done

echo ""
echo "=== Check 2: S3 files without metadata ==="
echo ""

S3_FILES=$(mktemp)
aws s3 ls "s3://$BUCKET_NAME/games/" --recursive | grep '\.z3$' | awk '{print $4}' > "$S3_FILES" || true

S3_COUNT=$(wc -l < "$S3_FILES" | tr -d ' ')
echo "Found $S3_COUNT .z3 files in S3"
echo ""

if [[ -s "$S3_FILES" ]]; then
    while read -r s3_key; do
        # Check if metadata exists
        METADATA_COUNT=$(aws dynamodb scan \
            --table-name "$TABLE_NAME" \
            --filter-expression "s3_key = :key" \
            --expression-attribute-values "{\":key\":{\"S\":\"$s3_key\"}}" \
            --select COUNT \
            --output json | jq -r '.Count')

        if [[ "$METADATA_COUNT" -gt 0 ]]; then
            echo "✓ $s3_key: Metadata exists"
        else
            echo "✗ ORPHAN: $s3_key"
            echo "  S3 file exists but no metadata in DynamoDB"
            ORPHANS_FOUND=$((ORPHANS_FOUND + 1))

            if [[ "$DELETE_MODE" == true ]]; then
                echo "  Deleting S3 file..."
                aws s3 rm "s3://$BUCKET_NAME/$s3_key"
                echo "  ✓ Deleted S3 file: $s3_key"
            fi
        fi
    done < "$S3_FILES"
else
    echo "No .z3 files found in S3"
fi

# Cleanup temp files
rm -f "$METADATA_ORPHANS" "$S3_FILES"

echo ""
echo "================================================"
echo "Summary"
echo "================================================"
echo "Total orphaned games found: $ORPHANS_FOUND"

if [[ "$ORPHANS_FOUND" -gt 0 ]]; then
    if [[ "$DELETE_MODE" == true ]]; then
        echo "✓ All orphans deleted"
    else
        echo "Run with --delete to remove orphans"
    fi
    exit 1
else
    echo "✓ No orphans found - all games clean"
    exit 0
fi
