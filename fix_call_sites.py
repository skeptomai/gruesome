#!/usr/bin/env python3
import re
import sys

def fix_emit_instruction_typed_calls(content):
    """
    Add None as the 5th parameter to emit_instruction_typed calls.
    This looks for the pattern: emit_instruction_typed(...)?; 
    and adds , None before the )?;
    """
    
    # Pattern to match emit_instruction_typed calls ending with )?;
    # This is more conservative - only matches complete calls
    pattern = r'(\.emit_instruction_typed\([^)]*\))(\?\;)'
    
    def add_none_parameter(match):
        call_part = match.group(1)
        ending = match.group(2)
        # Add , None before the closing )
        return call_part[:-1] + ', None)' + ending
    
    return re.sub(pattern, add_none_parameter, content, flags=re.MULTILINE | re.DOTALL)

if __name__ == "__main__":
    filename = sys.argv[1]
    with open(filename, 'r') as f:
        content = f.read()
    
    original_content = content
    fixed_content = fix_emit_instruction_typed_calls(content)
    
    if content != fixed_content:
        with open(filename, 'w') as f:
            f.write(fixed_content)
        print(f"Fixed {filename}")
        
        # Count how many changes were made
        original_calls = len(re.findall(r'\.emit_instruction_typed\([^)]*\)\?\;', original_content))
        fixed_calls = len(re.findall(r'\.emit_instruction_typed\([^)]*\)\?\;', fixed_content))
        print(f"  Updated {original_calls} call sites")
    else:
        print(f"No changes needed in {filename}")
