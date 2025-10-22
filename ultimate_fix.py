#!/usr/bin/env python3
import re

def count_params(call_text):
    """Count parameters in an emit_instruction_typed call"""
    # Remove the function name part
    content = call_text[call_text.find('(') + 1:call_text.rfind(')')]

    # Simple parameter counting by commas, accounting for nested structures
    depth = 0
    param_count = 1 if content.strip() else 0

    for char in content:
        if char in '([{':
            depth += 1
        elif char in ')]}':
            depth -= 1
        elif char == ',' and depth == 0:
            param_count += 1

    return param_count

def fix_function_calls(content):
    """Fix emit_instruction_typed calls to have exactly 5 parameters"""

    # Pattern to match emit_instruction_typed calls (including multiline)
    pattern = r'\bemit_instruction_typed\s*\([^}]*?\)\s*\?\s*;'

    def fix_call(match):
        call = match.group(0)
        param_count = count_params(call)

        if param_count == 3:
            # Add , None, None before )?;
            return call.replace(')?;', ', None, None)?;')
        elif param_count == 4:
            # Add , None before )?;
            return call.replace(')?;', ', None)?;')
        elif param_count > 5:
            # Remove extra parameters - this is tricky, so we'll do a simple fix
            # Replace common patterns with too many Nones
            fixed = call
            fixed = re.sub(r', None, None, None,\s*None', ', None, None', fixed)
            fixed = re.sub(r', None,\s*None,\s*None,\s*None,\s*None', ', None, None', fixed)
            return fixed
        else:
            return call

    return re.sub(pattern, fix_call, content, flags=re.MULTILINE | re.DOTALL)

# Apply to all files
for filename in ["src/grue_compiler/codegen.rs", "src/grue_compiler/codegen_instructions.rs", "src/grue_compiler/codegen_builtins.rs"]:
    try:
        with open(filename, 'r') as f:
            content = f.read()

        fixed = fix_function_calls(content)

        if content != fixed:
            with open(filename, 'w') as f:
                f.write(fixed)
            print(f"Fixed {filename}")
        else:
            print(f"No changes to {filename}")
    except Exception as e:
        print(f"Error with {filename}: {e}")