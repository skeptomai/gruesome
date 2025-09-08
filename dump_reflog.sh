#!/bin/bash
# Script to dump all reflog entries with timestamps and commit messages

echo "=== COMPLETE REFLOG DUMP ==="
echo "Format: [REFLOG_DATE] COMMIT_HASH REFLOG_ACTION -> COMMIT_DATE COMMIT_MESSAGE"
echo ""

git reflog --all --date=iso --pretty=format:"%gd %H %gs -> %ad %s" --date=iso