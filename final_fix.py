#!/usr/bin/env python3
import re
import sys

def final_fix_all_calls(content):
    """
    Final comprehensive fix for all emit_instruction_typed calls.
    Ensures exactly 5 parameters: opcode, operands, store_var, branch_offset, target_label_id
    """

    # Step 1: Fix calls with 3 parameters (missing branch_offset and target_label_id)
    # Pattern: emit_instruction_typed(opcode, operands, store_var)?;
    content = re.sub(
        r'(\bemit_instruction_typed\(\s*[^,]+,\s*[^,]+,\s*[^,]+)\s*\)(\?\s*;)',
        r'\1, None, None)\2',
        content
    )

    # Step 2: Fix calls with 4 parameters (missing target_label_id)
    # Pattern: emit_instruction_typed(opcode, operands, store_var, branch_offset)?;
    content = re.sub(
        r'(\bemit_instruction_typed\(\s*[^,]+,\s*[^,]+,\s*[^,]+,\s*[^,)]+)\s*\)(\?\s*;)',
        r'\1, None)\2',
        content
    )

    # Step 3: Fix multiline calls missing parameters
    # Look for closing )?; patterns and add missing None parameters before them

    # Fix multiline calls with only 3 parameters
    content = re.sub(
        r'(\bemit_instruction_typed\([^)]*?[^,]\s*)\n(\s*\)\?\s*;)',
        r'\1,\n\2None, // branch_offset\n\2None, // target_label_id\n\2',
        content,
        flags=re.MULTILINE | re.DOTALL
    )

    # Step 4: Remove extra parameters (calls with 6+ parameters)
    # Look for patterns with too many None parameters
    content = re.sub(
        r'(\bemit_instruction_typed\([^)]*?None,\s*//[^,\n]*\n[^)]*?)None,\s*//[^,\n]*(\n[^)]*?\)\?\s*;)',
        r'\1\2',
        content,
        flags=re.MULTILINE | re.DOTALL
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
            fixed_content = final_fix_all_calls(content)

            if content != fixed_content:
                with open(filename, 'w') as f:
                    f.write(fixed_content)
                print(f"Final fix applied to {filename}")
            else:
                print(f"No final fix needed in {filename}")
        except FileNotFoundError:
            print(f"File not found: {filename}")