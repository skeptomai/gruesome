#!/usr/bin/env python3
import os
import re

def fix_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Find all emit_instruction_typed calls and add the missing parameter
    # This pattern looks for the method call and adds None as the 5th parameter
    original_content = content
    
    # Pattern 1: Multi-line calls ending with )?;
    content = re.sub(
        r'(\w+\.emit_instruction_typed\([^)]*?\s*)(None,?)(\s*\)\?\;)',
        r'\1\2,\n            None // target_label_id\3',
        content,
        flags=re.MULTILINE | re.DOTALL
    )
    
    # Pattern 2: Single line calls
    content = re.sub(
        r'(\w+\.emit_instruction_typed\([^)]*?)(None)\)?;',
        r'\1\2, None)?;',
        content,
        flags=re.MULTILINE
    )
    
    if content != original_content:
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"Fixed: {filepath}")
    else:
        print(f"No changes: {filepath}")

# Fix the main files that have emit_instruction_typed calls
files_to_fix = [
    'src/grue_compiler/codegen.rs',
    'src/grue_compiler/codegen_builtins.rs', 
    'src/grue_compiler/codegen_instructions.rs',
    'src/grue_compiler/opcode_form_unit_tests.rs'
]

for filepath in files_to_fix:
    if os.path.exists(filepath):
        fix_file(filepath)
