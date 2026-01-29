# Orphan Detection Scripts

Scripts to identify and optionally delete orphaned resources in the Gruesome platform.

## Scripts

### check-orphaned-games.sh

Checks for orphaned game resources:
- **Metadata without S3 file**: DynamoDB has game metadata but S3 file is missing
- **S3 file without metadata**: S3 has game file but no DynamoDB metadata

**Usage:**
```bash
# Check staging (report only)
./scripts/check-orphaned-games.sh staging

# Check production (report only)
./scripts/check-orphaned-games.sh production

# Check and delete orphans in staging
./scripts/check-orphaned-games.sh staging --delete

# Check and delete orphans in production
./scripts/check-orphaned-games.sh production --delete
```

**Exit codes:**
- `0` - No orphans found
- `1` - Orphans found (or deleted if --delete flag used)

### check-orphaned-saves.sh

Checks for orphaned save resources:
- **Save metadata without S3 file**: DynamoDB has save metadata but S3 file is missing
- **Saves for non-existent users**: Save references a user_id that no longer exists
- **Saves for non-existent games**: Save references a game_id that doesn't exist
- **S3 file without metadata**: S3 has save file but no DynamoDB metadata

**Usage:**
```bash
# Check staging (report only)
./scripts/check-orphaned-saves.sh staging

# Check production (report only)
./scripts/check-orphaned-saves.sh production

# Check and delete orphans in staging
./scripts/check-orphaned-saves.sh staging --delete

# Check and delete orphans in production
./scripts/check-orphaned-saves.sh production --delete
```

**Exit codes:**
- `0` - No orphans found
- `1` - Orphans found (or deleted if --delete flag used)

## Examples

### Report only (safe to run anytime)
```bash
# Check all orphans in both environments
./scripts/check-orphaned-games.sh staging
./scripts/check-orphaned-games.sh production
./scripts/check-orphaned-saves.sh staging
./scripts/check-orphaned-saves.sh production
```

### Delete orphans
```bash
# Delete orphans in staging only
./scripts/check-orphaned-games.sh staging --delete
./scripts/check-orphaned-saves.sh staging --delete

# Delete orphans in production (use with caution!)
./scripts/check-orphaned-games.sh production --delete
./scripts/check-orphaned-saves.sh production --delete
```

## When to Run

**Recommended schedule:**
- Run checks weekly or after major data operations
- Always check staging before production
- Run without `--delete` first to review what will be removed

**Situations that may create orphans:**
- Failed uploads (S3 file created but metadata write fails)
- Failed deletions (metadata deleted but S3 file deletion fails)
- User deletion (saves may remain for deleted users)
- Game removal (saves may remain for removed games)
- Manual cleanup operations
- Database rollbacks or restores

## Safety Features

- **Report mode by default**: Scripts only report orphans unless `--delete` is specified
- **Environment isolation**: Must explicitly specify staging or production
- **Detailed output**: Shows exactly what will be deleted before deletion
- **Exit codes**: Can be used in CI/CD pipelines

## What Gets Deleted

### check-orphaned-games.sh --delete
- DynamoDB metadata records without S3 files
- S3 files without DynamoDB metadata

### check-orphaned-saves.sh --delete
- DynamoDB save metadata without S3 files
- DynamoDB save metadata for non-existent users (+ S3 file if exists)
- DynamoDB save metadata for non-existent games (+ S3 file if exists)
- S3 save files without DynamoDB metadata

## Integration with CI/CD

```bash
# Add to deployment verification
if ! ./scripts/check-orphaned-games.sh production; then
    echo "Warning: Orphaned games detected"
fi

if ! ./scripts/check-orphaned-saves.sh production; then
    echo "Warning: Orphaned saves detected"
fi
```

## Troubleshooting

**No orphans found but data seems inconsistent:**
- Verify AWS credentials are correct
- Check table/bucket names match environment
- Ensure IAM permissions allow scan/list operations

**Script hangs:**
- Large tables may take time to scan
- Check AWS service status
- Verify network connectivity

**Permission errors:**
- Requires DynamoDB scan/get/delete permissions
- Requires S3 list/get/delete permissions
- Check IAM role/user permissions
