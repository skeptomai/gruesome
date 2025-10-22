#!/usr/bin/env python3
import re
import sys

def simple_fix(content):
    """
    Simple targeted fix for emit_instruction_typed calls:
    1. 3 params -> add , None, None
    2. 4 params -> add , None
    """

    # Fix single-line calls with 3 params: emit_instruction_typed(a, b, c)?;
    content = re.sub(
        r'\bemit_instruction_typed\(([^,]+,\s*[^,]+,\s*[^,)]+)\)\?\s*;',
        r'emit_instruction_typed(\1, None, None)?;',
        content
    )

    # Fix single-line calls with 4 params: emit_instruction_typed(a, b, c, d)?;
    content = re.sub(
        r'\bemit_instruction_typed\(([^,]+,\s*[^,]+,\s*[^,]+,\s*[^,)]+)\)\?\s*;',
        r'emit_instruction_typed(\1, None)?;',
        content
    )

    # Fix multiline calls ending with just )?; - add None, None before closing
    content = re.sub(
        r'(\bemit_instruction_typed\([^)]*?[^,\s])\s*\n\s*\)\?\s*;',
        r'\1,\n                None, // branch_offset\n                None, // target_label_id\n            )?;',
        content,
        flags=re.MULTILINE | re.DOTALL
    )

    return content

if __name__ == "__main__":
    for filename in ["src/grue_compiler/codegen.rs", "src/grue_compiler/codegen_instructions.rs", "src/grue_compiler/codegen_builtins.rs"]:
        try:
            with open(filename, 'r') as f:
                content = f.read()

            fixed = simple_fix(content)

            if content != fixed:
                with open(filename, 'w') as f:
                    f.write(fixed)
                print(f"Fixed {filename}")
            else:
                print(f"No changes in {filename}")
        except Exception as e:
            print(f"Error with {filename}: {e}")