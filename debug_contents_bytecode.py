#!/usr/bin/env python3
"""Quick bytecode decoder for .contents() issue debugging"""

# Instruction bytes from xxd output
bytecode = [
    0xe0, 0x3f, 0x01, 0x9f, 0x00,  # @ 0x0338
    0xb0,                           # @ 0x033d  
    0x00,                           # @ 0x033e
    0x8d, 0x01, 0xb1,              # @ 0x033f
    0xe3, 0x13, 0x00, 0x01, 0x03, 0x00, 0x02,  # @ 0x0342
    0x11, 0x02, 0x01, 0x00,        # @ 0x0349
    0x4e,                           # @ 0x034d
    0x00,                           # @ 0x034e
    0x81, 0x00, 0x01,              # @ 0x034f
    0x81, 0x00, 0x01,              # @ 0x0352
    0x00,                           # @ 0x0355
    0xb1                            # @ 0x0356
]

pc = 0x0338
i = 0

while i < len(bytecode):
    addr = pc + i
    op = bytecode[i]
    
    print(f"{addr:04x}: {op:02x} ", end="")
    
    if op == 0xe0:  # VAR call_vs
        print(f"call_vs 0x{(bytecode[i+1] << 8) | bytecode[i+2]:04x} -> store var {bytecode[i+4]}")
        i += 5
    elif op == 0xb0:  # rtrue
        print("rtrue")
        i += 1
    elif op == 0x8d:  # print_paddr
        print(f"print_paddr 0x{(bytecode[i+1] << 8) | bytecode[i+2]:04x}")
        i += 3
    elif op == 0xe3:  # VAR put_prop 
        # operand types in byte i+1: 0x13 = 00|01|00|11 = Large|Small|Large|omitted
        # operand 1 (obj): bytes i+2,i+3 = Large constant
        # operand 2 (prop): byte i+4 = Small constant  
        # operand 3 (value): bytes i+5,i+6 = Large constant
        obj = (bytecode[i+2] << 8) | bytecode[i+3] 
        prop = bytecode[i+4]
        value = (bytecode[i+5] << 8) | bytecode[i+6]
        print(f"put_prop obj={obj}, prop={prop}, value=0x{value:04x}")
        i += 7
    elif op == 0x11:  # 2OP get_prop
        print(f"get_prop obj={bytecode[i+1]:02x}, prop={bytecode[i+2]} -> store var {bytecode[i+3]}")
        i += 4
    elif op == 0x4e:  # 1OP jz with branch offset
        print(f"jz -> branch offset {bytecode[i+1]:02x}")
        i += 2
    elif op == 0x81:  # 1OP store
        print(f"store {bytecode[i+1]:02x} -> var {bytecode[i+2]}")  
        i += 3
    elif op == 0x00:  # padding or data
        print("padding/data")
        i += 1
    else:
        print(f"UNKNOWN OPCODE")
        i += 1