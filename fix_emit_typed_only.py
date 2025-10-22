#!/usr/bin/env python3
import re
import sys

def fix_emit_typed_calls(content):
    """
    ONLY fix emit_instruction_typed calls, not emit_instruction calls.
    Add target_label_id: None parameter to emit_instruction_typed calls.
    """
    
    # Use a regex that specifically matches emit_instruction_typed calls
    # Pattern: look for self.emit_instruction_typed( followed by parameters ending with )?;
    pattern = r'(self\.emit_instruction_typed\((?:[^)]|\n)*?\s+)(None,?)(\s*\)\?\;)'
    
    def replace_match(match):
        prefix = match.group(1)
        last_none = match.group(2)
        suffix = match.group(3)
        
        # Add the new parameter
        if last_none.endswith(','):
            # Already has comma
            return prefix + last_none + "\n            None, // target_label_id" + suffix
        else:
            # Add comma and new parameter
            return prefix + last_none + ",\n            None, // target_label_id" + suffix
    
    return re.sub(pattern, replace_match, content, flags=re.MULTILINE | re.DOTALL)

if __name__ == "__main__":
    filename = sys.argv[1]
    with open(filename, 'r') as f:
        content = f.read()
    
    fixed_content = fix_emit_typed_calls(content)
    
    with open(filename, 'w') as f:
        f.write(fixed_content)
    
    print(f"Fixed {filename}")
