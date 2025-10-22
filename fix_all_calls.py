#!/usr/bin/env python3
import re
import sys

def fix_all_emit_calls(content):
    """
    Fix all emit_instruction_typed calls to have exactly 5 parameters.

    1. Add missing 5th parameter (None) to 4-parameter calls
    2. Remove extra parameters from calls with more than 5 parameters
    """

    # Pattern 1: Single-line calls with 4 parameters - add None
    # emit_instruction_typed(..., None)?;
    content = re.sub(
        r'(\bemit_instruction_typed\([^)]*?None\s*)\)(\?\s*;)',
        r'\1, None)\2',
        content,
        flags=re.MULTILINE
    )

    # Pattern 2: Multiline calls ending with ), )?; - add None parameter
    # Handles cases like:
    #   Some(2),
    # )?;
    content = re.sub(
        r'(\s+)(Some\(\d+\),)(\s+\)\?\s*;)',
        r'\1\2\1None, // target_label_id\3',
        content,
        flags=re.MULTILINE
    )

    # Pattern 3: Multiline calls ending with None, followed by )?;
    # Need to add another None parameter
    content = re.sub(
        r'(\s+)(None,\s*//[^\n]*\n\s*)\)(\?\s*;)',
        r'\1\2None, // target_label_id\n\1)\3',
        content,
        flags=re.MULTILINE
    )

    # Pattern 4: Remove extra parameters (6+ parameters)
    # Look for calls with None, None, None at the end
    content = re.sub(
        r'(\bemit_instruction_typed\([^)]*?None,\s*None,\s*)None,\s*([^)]*)\)(\?\s*;)',
        r'\1\2)\3',
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
            fixed_content = fix_all_emit_calls(content)

            if content != fixed_content:
                with open(filename, 'w') as f:
                    f.write(fixed_content)
                print(f"Fixed {filename}")
            else:
                print(f"No changes needed in {filename}")
        except FileNotFoundError:
            print(f"File not found: {filename}")