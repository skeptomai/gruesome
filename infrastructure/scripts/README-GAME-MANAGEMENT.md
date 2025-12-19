# Game Library Management Scripts

Scripts for maintaining consistency between S3 game files and DynamoDB metadata.

## Scripts

### 1. `verify-game-library.sh` - Verification Script

Verifies consistency between S3 files and DynamoDB metadata.

**Usage:**
```bash
./infrastructure/scripts/verify-game-library.sh [staging|prod]
```

**What it checks:**
1. **DynamoDB → S3 Consistency**
   - Every DynamoDB game entry has a corresponding S3 file
   - File sizes match between metadata and actual files
   - S3 key format is correct (`games/{game_id}.z3`)

2. **S3 → DynamoDB Consistency**
   - Every S3 game file has a corresponding DynamoDB entry

3. **S3 Key Format Consistency**
   - All s3_key values use the `games/` prefix

**Example:**
```bash
# Verify production
./infrastructure/scripts/verify-game-library.sh prod

# Verify staging
./infrastructure/scripts/verify-game-library.sh staging
```

**Output:**
- ✓ Green checkmarks for passing checks
- ✗ Red X for errors
- ⚠ Yellow warnings for size mismatches

**Exit codes:**
- 0: All checks passed (or warnings only)
- 1: Errors found

---

### 2. `upload-game.sh` - Standardized Upload Script

Uploads a game file to S3 and creates DynamoDB metadata with consistent formatting.

**Usage:**
```bash
./infrastructure/scripts/upload-game.sh <game_file> <game_id> <title> <author> <description> [staging|prod]
```

**Arguments:**
- `game_file` - Path to .z3/.z4/.z5 game file
- `game_id` - Unique identifier (e.g., 'zork1', 'hhgg')
- `title` - Full game title (quoted)
- `author` - Author name(s) (quoted)
- `description` - Brief description (quoted)
- `environment` - 'staging' or 'prod' (default: staging)

**Example:**
```bash
./infrastructure/scripts/upload-game.sh \
  resources/test/zork1/DATA/ZORK1.DAT \
  zork1 \
  "Zork I: The Great Underground Empire" \
  "Marc Blank & Dave Lebling" \
  "The original classic adventure game. Explore the Great Underground Empire." \
  staging
```

**What it does:**
1. Validates the game file exists and is a valid Z-Machine file
2. Extracts Z-Machine version automatically
3. Calculates file size
4. Confirms upload with user
5. Uploads to S3 with correct path: `s3://{bucket}/games/{game_id}.z{version}`
6. Creates DynamoDB metadata with standardized format
7. Verifies both uploads succeeded
8. Provides next steps

**Safety features:**
- Validates Z-Machine version (1-8)
- Requires confirmation before upload
- Verifies uploads after completion
- Shows stored metadata for review

---

## Workflow

### Uploading a New Game

1. **Upload to staging first:**
   ```bash
   ./infrastructure/scripts/upload-game.sh \
     path/to/game.z3 \
     game_id \
     "Game Title" \
     "Author Name" \
     "Description" \
     staging
   ```

2. **Verify staging:**
   ```bash
   ./infrastructure/scripts/verify-game-library.sh staging
   ```

3. **Test the game in staging:**
   - Visit https://staging.gruesome.skeptomai.com
   - Load the game and verify it works

4. **Upload to production:**
   ```bash
   ./infrastructure/scripts/upload-game.sh \
     path/to/game.z3 \
     game_id \
     "Game Title" \
     "Author Name" \
     "Description" \
     prod
   ```

5. **Verify production:**
   ```bash
   ./infrastructure/scripts/verify-game-library.sh prod
   ```

### Regular Maintenance

Run verification periodically to catch any inconsistencies:
```bash
# Check both environments
./infrastructure/scripts/verify-game-library.sh staging
./infrastructure/scripts/verify-game-library.sh prod
```

---

## Metadata Format

The scripts ensure consistent metadata format:

```json
{
  "PK": "GAME#<game_id>",
  "SK": "METADATA",
  "entity_type": "GAME",
  "game_id": "<game_id>",
  "title": "<title>",
  "author": "<author>",
  "description": "<description>",
  "version": <z-machine-version>,
  "file_size": <bytes>,
  "s3_key": "games/<game_id>.z<version>",
  "created_at": <unix-timestamp>
}
```

**Critical fields:**
- `s3_key` MUST always start with `games/`
- `version` is auto-detected from game file
- `file_size` is calculated from actual file
- All game files stored in `games/` directory in S3

---

## Troubleshooting

### S3 file missing
```bash
# Re-upload the file
aws s3 cp game.z3 s3://gruesome-games/games/game_id.z3
```

### DynamoDB entry missing
```bash
# Use upload script to recreate (it will overwrite S3 file)
./infrastructure/scripts/upload-game.sh <file> <id> <title> <author> <desc> prod
```

### S3 key format incorrect
```bash
# Update metadata
aws dynamodb update-item \
  --table-name gruesome-platform \
  --key '{"PK": {"S": "GAME#game_id"}, "SK": {"S": "METADATA"}}' \
  --update-expression "SET s3_key = :key" \
  --expression-attribute-values '{":key": {"S": "games/game_id.z3"}}'
```

---

## Prevention Checklist

✅ Always use `upload-game.sh` script for new uploads
✅ Run `verify-game-library.sh` after uploads
✅ Test games in staging before production
✅ Run verification periodically (weekly recommended)
✅ Never manually edit DynamoDB without verifying S3
✅ Never manually upload to S3 without creating metadata
