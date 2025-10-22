#!/usr/bin/env python3
import re
import sys

def fix_emit_typed_calls(filename):
    with open(filename, 'r') as f:
        content = f.read()
    
    # Simple pattern: find emit_instruction_typed calls and add the 5th parameter
    # Look for the pattern: emit_instruction_typed(..., None,\n        )?;
    pattern = r'(self\.emit_instruction_typed\([^)]*?),(\s*\n\s*\)\?\;)'
    
    def add_param(match):
        call_content = match.group(1)
        closing = match.group(2)
        # Add the new parameter
        return call_content + ",\n            None, // target_label_id" + closing
    
    fixed_content = re.sub(pattern, add_param, content, flags=re.MULTILINE | re.DOTALL)
    
    with open(filename, 'w') as f:
        f.write(fixed_content)
    
    print(f"Fixed {filename}")

if __name__ == "__main__":
    fix_emit_typed_calls(sys.argv[1])
