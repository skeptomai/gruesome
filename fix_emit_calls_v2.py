#!/usr/bin/env python3
import re
import sys

def fix_emit_calls(content):
    """
    Fix emit_instruction_typed calls to add the new target_label_id parameter.
    
    This needs to handle various patterns:
    1. Simple: None,\n)?;
    2. With comments: None, // comment\n)?;
    3. With long comments: None,                      // comment\n)?;
    """
    
    # Split content into lines for easier processing
    lines = content.split('\n')
    new_lines = []
    i = 0
    
    while i < len(lines):
        line = lines[i]
        
        # Check if this line contains emit_instruction_typed
        if 'emit_instruction_typed(' in line:
            # This is the start of an emit_instruction_typed call
            # Find the closing )?; line
            call_lines = [line]
            i += 1
            
            while i < len(lines) and not (lines[i].strip().endswith(')?;') or lines[i].strip() == ')?;'):
                call_lines.append(lines[i])
                i += 1
            
            # Add the closing line
            if i < len(lines):
                call_lines.append(lines[i])
                
                # Check if this call needs fixing by counting parameters
                # Look for the pattern where we have 4 parameters ending with None
                call_text = '\n'.join(call_lines)
                
                # Count commas to estimate parameters (rough heuristic)
                # Look for the last meaningful line before )?;
                for j in range(len(call_lines) - 2, -1, -1):
                    line_content = call_lines[j].strip()
                    if line_content and not line_content.startswith('//'):
                        # This is likely the last parameter line
                        if ('None,' in line_content or 
                            (line_content.endswith(',') and ('None' in line_content or 'Some(' in line_content))):
                            # This looks like a parameter line that needs fixing
                            # Add the new parameter before the closing
                            closing_line = call_lines[-1]
                            indent = re.match(r'(\s*)', closing_line).group(1)
                            call_lines.insert(-1, f"{indent}None, // target_label_id")
                        break
                
                new_lines.extend(call_lines)
            else:
                new_lines.extend(call_lines)
        else:
            new_lines.append(line)
            i += 1
    
    return '\n'.join(new_lines)

if __name__ == "__main__":
    filename = sys.argv[1]
    with open(filename, 'r') as f:
        content = f.read()
    
    fixed_content = fix_emit_calls(content)
    
    with open(filename, 'w') as f:
        f.write(fixed_content)
    
    print(f"Fixed {filename}")
