#!/usr/bin/env python3
import re
import sys

def cleanup_duplicates(content):
    """
    Remove duplicate None, // target_label_id parameters from emit_instruction_typed calls.
    """

    # Pattern 1: Remove duplicate None, // target_label_id lines
    # Match cases where we have two consecutive "None, // target_label_id" lines
    content = re.sub(
        r'(\s+None,\s*//\s*target_label_id\s*\n\s*)None,\s*//\s*target_label_id',
        r'\1',
        content,
        flags=re.MULTILINE
    )

    # Pattern 2: Remove extra None parameters at the end of function calls
    # Match cases where we have multiple consecutive None parameters
    content = re.sub(
        r'(None,\s*//\s*target_label_id\s*)\s*None,?\s*(\s*\))',
        r'\1\2',
        content,
        flags=re.MULTILINE
    )

    # Pattern 3: Fix calls with 6 parameters - remove one None
    # emit_instruction_typed(..., None, None, // target_label_id
    content = re.sub(
        r'(\s+None,\s*None,\s*)\s*None,\s*//\s*target_label_id',
        r'\1None, // target_label_id',
        content,
        flags=re.MULTILINE
    )

    # Pattern 4: Fix calls where we have None, followed by duplicate None, // target_label_id
    content = re.sub(
        r'(\s+None,\s*\n\s*)None,\s*//\s*target_label_id\s*\n\s*None,\s*//\s*target_label_id',
        r'\1None, // target_label_id',
        content,
        flags=re.MULTILINE
    )

    # Pattern 5: Clean up any remaining multiple consecutive None parameters
    content = re.sub(
        r'(\s+None,\s*)(\s*None,\s*)*(\s*//\s*target_label_id)',
        r'\1\3',
        content,
        flags=re.MULTILINE
    )

    return content

if __name__ == "__main__":
    filenames = [
        "src/grue_compiler/codegen.rs",
        "src/grue_compiler/codegen_instructions.rs",
        "src/grue_compiler/codegen_builtins.rs"
    ]

    for filename in filenames:
        try:
            with open(filename, 'r') as f:
                content = f.read()

            original_content = content
            cleaned_content = cleanup_duplicates(content)

            if content != cleaned_content:
                with open(filename, 'w') as f:
                    f.write(cleaned_content)
                print(f"Cleaned duplicates in {filename}")
            else:
                print(f"No cleanup needed in {filename}")
        except FileNotFoundError:
            print(f"File not found: {filename}")