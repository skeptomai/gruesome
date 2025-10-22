#!/usr/bin/env python3
import re
import sys

def fix_emit_calls(content):
    """
    Comprehensive fix for emit_instruction and emit_instruction_typed calls.

    1. emit_instruction_typed calls missing 5th parameter: add ", None"
    2. emit_instruction calls with extra 5th parameter: remove it
    3. emit_instruction_typed calls with 6 parameters: remove extra one
    """

    original_content = content

    # Pattern 1: Fix emit_instruction_typed calls missing the 5th parameter
    # Look for: emit_instruction_typed(..., None)?; (ending with 4 parameters)
    # This matches both single-line and multiline patterns
    content = re.sub(
        r'(\.emit_instruction_typed\([^)]*?\s*None,?\s*)((?:\s*//[^\n]*)?\s*\)\?;)',
        r'\1,\n            None // target_label_id\2',
        content,
        flags=re.MULTILINE | re.DOTALL
    )

    # Pattern 2: Fix emit_instruction calls with extra 5th parameter
    # Look for: emit_instruction(..., None, None, None, // target_label_id)?;
    # Remove the extra ", None // target_label_id" part
    content = re.sub(
        r'(\.emit_instruction\([^)]*?None,\s*None,\s*)\s*None,?\s*//\s*target_label_id\s*(\s*\)\?;)',
        r'\1\2',
        content,
        flags=re.MULTILINE | re.DOTALL
    )

    # Pattern 3: Fix emit_instruction calls with extra parameter (simpler case)
    # Look for: emit_instruction(..., None, None, None)?; where the last None shouldn't be there
    content = re.sub(
        r'(\.emit_instruction\([^)]*?None,\s*None,\s*)\s*None,?\s*(\s*\)\?;)',
        r'\1\2',
        content,
        flags=re.MULTILINE | re.DOTALL
    )

    return content

if __name__ == "__main__":
    filename = sys.argv[1]
    with open(filename, 'r') as f:
        content = f.read()

    original_content = content
    fixed_content = fix_emit_calls(content)

    if content != fixed_content:
        with open(filename, 'w') as f:
            f.write(fixed_content)
        print(f"Fixed {filename}")
    else:
        print(f"No changes needed in {filename}")