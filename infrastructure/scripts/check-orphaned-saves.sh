#!/bin/bash
set -e

# Check for orphaned saves in DynamoDB and S3
# Usage: ./check-orphaned-saves.sh [staging|production] [--delete]

ENVIRONMENT=${1:-staging}
DELETE_MODE=false

if [[ "$2" == "--delete" ]]; then
    DELETE_MODE=true
fi

# Set environment-specific variables
if [[ "$ENVIRONMENT" == "production" ]]; then
    TABLE_NAME="gruesome-platform"
    BUCKET_NAME="gruesome-saves"
elif [[ "$ENVIRONMENT" == "staging" ]]; then
    TABLE_NAME="gruesome-platform-staging"
    BUCKET_NAME="gruesome-saves-staging"
else
    echo "Error: Environment must be 'staging' or 'production'"
    exit 1
fi

echo "================================================"
echo "Checking Orphaned Saves: $ENVIRONMENT"
echo "================================================"
echo "Table: $TABLE_NAME"
echo "Bucket: $BUCKET_NAME"
echo "Delete Mode: $DELETE_MODE"
echo ""

ORPHANS_FOUND=0

# Check 1: DynamoDB save metadata without S3 files
echo "=== Check 1: Save metadata without S3 files ==="
echo ""

SAVE_METADATA=$(mktemp)
aws dynamodb scan \
    --table-name "$TABLE_NAME" \
    --filter-expression "begins_with(SK, :sk)" \
    --expression-attribute-values '{":sk":{"S":"SAVE#"}}' \
    --output json > "$SAVE_METADATA"

SAVE_COUNT=$(jq -r '.Count' "$SAVE_METADATA")
echo "Found $SAVE_COUNT saves in DynamoDB"
echo ""

if [[ "$SAVE_COUNT" -gt 0 ]]; then
    jq -c '.Items[]' "$SAVE_METADATA" | while read -r save; do
        PK=$(echo "$save" | jq -r '.PK.S')
        SK=$(echo "$save" | jq -r '.SK.S')
        S3_KEY=$(echo "$save" | jq -r '.s3_key.S // empty')
        GAME_ID=$(echo "$save" | jq -r '.game_id.S // empty')

        if [[ -z "$S3_KEY" ]]; then
            echo "✗ ORPHAN: $PK / $SK"
            echo "  Missing s3_key attribute"
            ORPHANS_FOUND=$((ORPHANS_FOUND + 1))
            continue
        fi

        # Check if S3 file exists
        if aws s3 ls "s3://$BUCKET_NAME/saves/$S3_KEY" >/dev/null 2>&1; then
            echo "✓ $PK / $SK: S3 file exists"
        else
            echo "✗ ORPHAN: $PK / $SK"
            echo "  Metadata exists but S3 file missing: saves/$S3_KEY"
            ORPHANS_FOUND=$((ORPHANS_FOUND + 1))

            if [[ "$DELETE_MODE" == true ]]; then
                echo "  Deleting metadata..."
                aws dynamodb delete-item \
                    --table-name "$TABLE_NAME" \
                    --key "{\"PK\":{\"S\":\"$PK\"},\"SK\":{\"S\":\"$SK\"}}"
                echo "  ✓ Deleted metadata"
            fi
        fi
    done
else
    echo "No saves found in DynamoDB"
fi

echo ""
echo "=== Check 2: Saves for non-existent users ==="
echo ""

if [[ "$SAVE_COUNT" -gt 0 ]]; then
    jq -c '.Items[]' "$SAVE_METADATA" | while read -r save; do
        PK=$(echo "$save" | jq -r '.PK.S')
        SK=$(echo "$save" | jq -r '.SK.S')

        # Extract user_id from PK (format: USER#<user_id>)
        USER_ID="${PK#USER#}"

        # Check if user profile exists
        USER_EXISTS=$(aws dynamodb get-item \
            --table-name "$TABLE_NAME" \
            --key "{\"PK\":{\"S\":\"USER#$USER_ID\"},\"SK\":{\"S\":\"PROFILE\"}}" \
            --output json | jq -r 'if .Item then "true" else "false" end')

        if [[ "$USER_EXISTS" == "true" ]]; then
            echo "✓ $PK / $SK: User exists"
        else
            echo "✗ ORPHAN: $PK / $SK"
            echo "  Save references non-existent user: $USER_ID"
            ORPHANS_FOUND=$((ORPHANS_FOUND + 1))

            if [[ "$DELETE_MODE" == true ]]; then
                echo "  Deleting save metadata..."
                aws dynamodb delete-item \
                    --table-name "$TABLE_NAME" \
                    --key "{\"PK\":{\"S\":\"$PK\"},\"SK\":{\"S\":\"$SK\"}}"

                # Also delete S3 file if it exists
                S3_KEY=$(echo "$save" | jq -r '.s3_key.S // empty')
                if [[ -n "$S3_KEY" ]]; then
                    if aws s3 ls "s3://$BUCKET_NAME/saves/$S3_KEY" >/dev/null 2>&1; then
                        aws s3 rm "s3://$BUCKET_NAME/saves/$S3_KEY"
                        echo "  ✓ Deleted S3 file"
                    fi
                fi
                echo "  ✓ Deleted save for non-existent user"
            fi
        fi
    done
else
    echo "No saves to check"
fi

echo ""
echo "=== Check 3: Saves for non-existent games ==="
echo ""

if [[ "$SAVE_COUNT" -gt 0 ]]; then
    jq -c '.Items[]' "$SAVE_METADATA" | while read -r save; do
        PK=$(echo "$save" | jq -r '.PK.S')
        SK=$(echo "$save" | jq -r '.SK.S')
        GAME_ID=$(echo "$save" | jq -r '.game_id.S // empty')

        if [[ -z "$GAME_ID" ]]; then
            echo "✗ ORPHAN: $PK / $SK"
            echo "  Missing game_id attribute"
            ORPHANS_FOUND=$((ORPHANS_FOUND + 1))
            continue
        fi

        # Check if game exists
        GAME_EXISTS=$(aws dynamodb get-item \
            --table-name "$TABLE_NAME" \
            --key "{\"PK\":{\"S\":\"GAME#$GAME_ID\"},\"SK\":{\"S\":\"METADATA\"}}" \
            --output json | jq -r 'if .Item then "true" else "false" end')

        if [[ "$GAME_EXISTS" == "true" ]]; then
            echo "✓ $PK / $SK: Game exists ($GAME_ID)"
        else
            echo "✗ ORPHAN: $PK / $SK"
            echo "  Save references non-existent game: $GAME_ID"
            ORPHANS_FOUND=$((ORPHANS_FOUND + 1))

            if [[ "$DELETE_MODE" == true ]]; then
                echo "  Deleting save metadata..."
                aws dynamodb delete-item \
                    --table-name "$TABLE_NAME" \
                    --key "{\"PK\":{\"S\":\"$PK\"},\"SK\":{\"S\":\"$SK\"}}"

                # Also delete S3 file if it exists
                S3_KEY=$(echo "$save" | jq -r '.s3_key.S // empty')
                if [[ -n "$S3_KEY" ]]; then
                    if aws s3 ls "s3://$BUCKET_NAME/saves/$S3_KEY" >/dev/null 2>&1; then
                        aws s3 rm "s3://$BUCKET_NAME/saves/$S3_KEY"
                        echo "  ✓ Deleted S3 file"
                    fi
                fi
                echo "  ✓ Deleted save for non-existent game"
            fi
        fi
    done
else
    echo "No saves to check"
fi

echo ""
echo "=== Check 4: S3 save files without metadata ==="
echo ""

S3_FILES=$(mktemp)
aws s3 ls "s3://$BUCKET_NAME/saves/" --recursive | grep '\.sav$' | awk '{print $4}' > "$S3_FILES" || true

S3_COUNT=$(wc -l < "$S3_FILES" | tr -d ' ')
echo "Found $S3_COUNT .sav files in S3"
echo ""

if [[ -s "$S3_FILES" ]]; then
    while read -r full_path; do
        # Extract s3_key (remove 'saves/' prefix)
        S3_KEY="${full_path#saves/}"

        # Check if metadata exists
        METADATA_COUNT=$(aws dynamodb scan \
            --table-name "$TABLE_NAME" \
            --filter-expression "s3_key = :key" \
            --expression-attribute-values "{\":key\":{\"S\":\"$S3_KEY\"}}" \
            --select COUNT \
            --output json | jq -r '.Count')

        if [[ "$METADATA_COUNT" -gt 0 ]]; then
            echo "✓ $S3_KEY: Metadata exists"
        else
            echo "✗ ORPHAN: $S3_KEY"
            echo "  S3 file exists but no metadata in DynamoDB"
            ORPHANS_FOUND=$((ORPHANS_FOUND + 1))

            if [[ "$DELETE_MODE" == true ]]; then
                echo "  Deleting S3 file..."
                aws s3 rm "s3://$BUCKET_NAME/$full_path"
                echo "  ✓ Deleted S3 file: $S3_KEY"
            fi
        fi
    done < "$S3_FILES"
else
    echo "No .sav files found in S3"
fi

# Cleanup temp files
rm -f "$SAVE_METADATA" "$S3_FILES"

echo ""
echo "================================================"
echo "Summary"
echo "================================================"
echo "Total orphaned saves found: $ORPHANS_FOUND"

if [[ "$ORPHANS_FOUND" -gt 0 ]]; then
    if [[ "$DELETE_MODE" == true ]]; then
        echo "✓ All orphans deleted"
    else
        echo "Run with --delete to remove orphans"
    fi
    exit 1
else
    echo "✓ No orphans found - all saves clean"
    exit 0
fi
