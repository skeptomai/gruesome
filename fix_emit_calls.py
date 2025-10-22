#!/usr/bin/env python3
import re
import sys

def fix_emit_calls(content):
    # Pattern to match emit_instruction_typed calls with 4 parameters ending in None,
    # This matches the pattern where we have:
    # emit_instruction_typed(
    #     ...
    #     None,
    # )?;
    # We want to change the last None, to None, None,
    
    # Look for lines that end with "None," and are followed by whitespace and ")?;"
    pattern = r'(\s+)(None,)(\s+\)\?\;)'
    
    def replace_match(match):
        indent = match.group(1)
        none_part = match.group(2)
        closing = match.group(3)
        # Add the new None parameter
        return indent + none_part + "\n" + indent + "None," + closing
    
    return re.sub(pattern, replace_match, content, flags=re.MULTILINE)

if __name__ == "__main__":
    filename = sys.argv[1]
    with open(filename, 'r') as f:
        content = f.read()
    
    fixed_content = fix_emit_calls(content)
    
    with open(filename, 'w') as f:
        f.write(fixed_content)
    
    print(f"Fixed {filename}")
