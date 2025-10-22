#!/usr/bin/env python3
import re
import sys

def fix_multiline_emit_calls(content):
    """
    Fix multiline emit_instruction_typed calls by adding None as 5th parameter.
    Pattern: emit_instruction_typed(...,\n        None,\n    )?;
    """
    
    # Pattern for multiline calls ending with None, followed by )?;
    # This looks for: None,\n[whitespace])?;
    pattern = r'(\s+None,)(\s*\)\?\;)'
    
    def add_none_parameter(match):
        none_line = match.group(1) 
        closing = match.group(2)
        
        # Extract the indentation from the None line
        indent_match = re.match(r'(\s+)', none_line)
        if indent_match:
            indent = indent_match.group(1)
        else:
            indent = '        '  # fallback
            
        # Add new parameter line with same indentation
        return none_line + '\n' + indent + 'None, // target_label_id' + closing
    
    return re.sub(pattern, add_none_parameter, content, flags=re.MULTILINE)

if __name__ == "__main__":
    filename = sys.argv[1]
    with open(filename, 'r') as f:
        content = f.read()
    
    original_content = content
    fixed_content = fix_multiline_emit_calls(content)
    
    if content != fixed_content:
        with open(filename, 'w') as f:
            f.write(fixed_content)
        print(f"Fixed {filename}")
        
        # Count changes by counting the pattern before and after
        original_count = len(re.findall(r'\s+None,\s*\)\?\;', original_content))
        new_count = len(re.findall(r'None, // target_label_id\s*\)\?\;', fixed_content))
        print(f"  Added target_label_id parameter to {new_count} multiline calls")
    else:
        print(f"No multiline changes needed in {filename}")
