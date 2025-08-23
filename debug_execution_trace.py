#!/usr/bin/env python3
"""
Z-Machine Execution Tracer
Analyzes compiled Z-Machine files to understand runtime execution paths and failures
"""

import sys
import struct
from typing import List, Dict, Tuple, Optional

class ZMachineAnalyzer:
    def __init__(self, filename: str):
        self.filename = filename
        with open(filename, 'rb') as f:
            self.data = f.read()
        
        # Parse Z-Machine header
        self.version = self.data[0]
        self.high_memory = struct.unpack('>H', self.data[4:6])[0]
        self.start_pc = struct.unpack('>H', self.data[6:8])[0]
        self.dictionary = struct.unpack('>H', self.data[8:10])[0]
        self.object_table = struct.unpack('>H', self.data[10:12])[0]
        self.global_variables = struct.unpack('>H', self.data[12:14])[0]
        self.static_memory = struct.unpack('>H', self.data[14:16])[0]
        
        print(f"Z-Machine file: {filename}")
        print(f"Version: {self.version}")
        print(f"Start PC: 0x{self.start_pc:04x}")
        print(f"High memory: 0x{self.high_memory:04x}")
        print(f"Static memory: 0x{self.static_memory:04x}")
        print(f"Object table: 0x{self.object_table:04x}")
        print(f"Dictionary: 0x{self.dictionary:04x}")
        print(f"Global vars: 0x{self.global_variables:04x}")
        print()

    def get_byte(self, addr: int) -> int:
        if addr >= len(self.data):
            return 0
        return self.data[addr]

    def get_word(self, addr: int) -> int:
        if addr + 1 >= len(self.data):
            return 0
        return struct.unpack('>H', self.data[addr:addr+2])[0]

    def decode_instruction_at(self, pc: int) -> Tuple[str, int, List[int], Optional[int]]:
        """Decode Z-Machine instruction at PC. Returns (opcode_name, next_pc, operands, store_var)"""
        if pc >= len(self.data):
            return "EOF", pc, [], None
            
        opcode_byte = self.get_byte(pc)
        
        # Determine instruction form
        if opcode_byte & 0x80 == 0:
            # Long form (2OP)
            return self.decode_long_form(pc)
        elif opcode_byte & 0x40 == 0:
            # Short form (1OP or 0OP)
            return self.decode_short_form(pc)
        else:
            # Variable form (VAR)
            return self.decode_variable_form(pc)

    def decode_long_form(self, pc: int) -> Tuple[str, int, List[int], Optional[int]]:
        opcode_byte = self.get_byte(pc)
        opcode = opcode_byte & 0x1F
        
        # Operand types for Long form
        op1_type = (opcode_byte & 0x40) >> 6  # 0=small constant, 1=variable
        op2_type = (opcode_byte & 0x20) >> 5  # 0=small constant, 1=variable
        
        operands = []
        next_pc = pc + 1
        
        # Decode operand 1
        if op1_type == 0:  # Small constant
            operands.append(self.get_byte(next_pc))
            next_pc += 1
        else:  # Variable
            operands.append(self.get_byte(next_pc))
            next_pc += 1
            
        # Decode operand 2
        if op2_type == 0:  # Small constant
            operands.append(self.get_byte(next_pc))
            next_pc += 1
        else:  # Variable
            operands.append(self.get_byte(next_pc))
            next_pc += 1
        
        # Some 2OP instructions store results
        store_var = None
        if opcode in [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A]:
            store_var = self.get_byte(next_pc)
            next_pc += 1
            
        opcode_names = {
            0x01: "je", 0x02: "jl", 0x03: "jg", 0x04: "dec_chk", 0x05: "inc_chk",
            0x06: "jin", 0x07: "test", 0x08: "or", 0x09: "and", 0x0A: "test_attr",
            0x0B: "set_attr", 0x0C: "clear_attr", 0x0D: "store", 0x0E: "insert_obj",
            0x0F: "loadw", 0x10: "loadb", 0x11: "get_prop", 0x12: "get_prop_addr",
            0x13: "get_next_prop", 0x14: "add", 0x15: "sub", 0x16: "mul", 0x17: "div",
            0x18: "mod", 0x19: "call_2s", 0x1A: "call_2n", 0x1B: "set_colour",
            0x1C: "throw"
        }
        
        name = opcode_names.get(opcode, f"unknown_2op_{opcode:02x}")
        return name, next_pc, operands, store_var

    def decode_short_form(self, pc: int) -> Tuple[str, int, List[int], Optional[int]]:
        opcode_byte = self.get_byte(pc)
        opcode = opcode_byte & 0x0F
        op_type = (opcode_byte & 0x30) >> 4  # 0=large const, 1=small const, 2=variable, 3=omitted
        
        operands = []
        next_pc = pc + 1
        
        if op_type == 0:  # Large constant
            operands.append(self.get_word(next_pc))
            next_pc += 2
        elif op_type == 1:  # Small constant
            operands.append(self.get_byte(next_pc))
            next_pc += 1
        elif op_type == 2:  # Variable
            operands.append(self.get_byte(next_pc))
            next_pc += 1
        # op_type == 3 means no operand (0OP)
        
        store_var = None
        if op_type != 3:  # 1OP instructions
            opcode_names = {
                0x00: "jz", 0x01: "get_sibling", 0x02: "get_child", 0x03: "get_parent",
                0x04: "get_prop_len", 0x05: "inc", 0x06: "dec", 0x07: "print_addr",
                0x08: "call_1s", 0x09: "remove_obj", 0x0A: "print_obj", 0x0B: "ret",
                0x0C: "jump", 0x0D: "print_paddr", 0x0E: "load", 0x0F: "not"
            }
            if opcode in [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x08, 0x0E, 0x0F]:
                store_var = self.get_byte(next_pc)
                next_pc += 1
        else:  # 0OP instructions
            opcode_names = {
                0x00: "rtrue", 0x01: "rfalse", 0x02: "print", 0x03: "print_ret",
                0x04: "nop", 0x05: "save", 0x06: "restore", 0x07: "restart",
                0x08: "ret_popped", 0x09: "pop", 0x0A: "quit", 0x0B: "new_line",
                0x0C: "show_status", 0x0D: "verify", 0x0E: "extended", 0x0F: "piracy"
            }
            
        name = opcode_names.get(opcode, f"unknown_{op_type}op_{opcode:02x}")
        return name, next_pc, operands, store_var

    def decode_variable_form(self, pc: int) -> Tuple[str, int, List[int], Optional[int]]:
        opcode_byte = self.get_byte(pc)
        opcode = opcode_byte & 0x1F
        
        # Get operand types byte
        types_byte = self.get_byte(pc + 1)
        next_pc = pc + 2
        
        operands = []
        for i in range(4):  # VAR instructions can have 0-4 operands
            op_type = (types_byte >> (6 - 2*i)) & 0x03
            if op_type == 3:  # Omitted operand
                break
            elif op_type == 0:  # Large constant
                operands.append(self.get_word(next_pc))
                next_pc += 2
            elif op_type == 1:  # Small constant
                operands.append(self.get_byte(next_pc))
                next_pc += 1
            elif op_type == 2:  # Variable
                operands.append(self.get_byte(next_pc))
                next_pc += 1
        
        # Many VAR instructions store results
        store_var = None
        if opcode in [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B]:
            store_var = self.get_byte(next_pc)
            next_pc += 1
            
        opcode_names = {
            0x00: "call_vs", 0x01: "storew", 0x02: "storeb", 0x03: "put_prop",
            0x04: "sread", 0x05: "print_char", 0x06: "print_num", 0x07: "random",
            0x08: "push", 0x09: "pull", 0x0A: "split_window", 0x0B: "set_window",
            0x0C: "call_vs2", 0x0D: "erase_window", 0x0E: "erase_line", 0x0F: "set_cursor",
            0x10: "get_cursor", 0x11: "set_text_style", 0x12: "buffer_mode", 0x13: "output_stream",
            0x14: "input_stream", 0x15: "sound_effect", 0x16: "read_char", 0x17: "scan_table",
            0x18: "not", 0x19: "call_vn", 0x1A: "call_vn2", 0x1B: "tokenise",
            0x1C: "encode_text", 0x1D: "copy_table", 0x1E: "print_table", 0x1F: "check_arg_count"
        }
        
        name = opcode_names.get(opcode, f"unknown_var_{opcode:02x}")
        return name, next_pc, operands, store_var

    def trace_from_start(self, max_instructions: int = 50):
        """Trace execution from start PC"""
        print(f"Tracing execution from PC 0x{self.start_pc:04x}")
        print("=" * 60)
        
        pc = self.start_pc
        for i in range(max_instructions):
            if pc >= len(self.data):
                print(f"PC out of bounds: 0x{pc:04x}")
                break
                
            opcode_name, next_pc, operands, store_var = self.decode_instruction_at(pc)
            
            # Format operands
            operands_str = ", ".join(f"0x{op:02x}" if op < 256 else f"0x{op:04x}" for op in operands)
            if not operands_str:
                operands_str = "none"
                
            store_str = f" -> var{store_var:02x}" if store_var is not None else ""
            
            print(f"{i:2d}: PC=0x{pc:04x} {opcode_name}({operands_str}){store_str}")
            
            # Check for dangerous areas
            if pc > self.high_memory:
                print(f"    WARNING: PC in high memory area (> 0x{self.high_memory:04x})")
            elif pc > self.static_memory:
                print(f"    INFO: PC in static memory area")
                
            # Show raw bytes for debugging
            raw_bytes = [f"{self.get_byte(pc + j):02x}" for j in range(min(8, next_pc - pc))]
            print(f"    Raw bytes: {' '.join(raw_bytes)}")
            
            # Special instruction handling
            if opcode_name == "rtrue":
                print("    Execution would return TRUE")
                break
            elif opcode_name == "rfalse":
                print("    Execution would return FALSE")
                break
            elif opcode_name == "quit":
                print("    Execution would quit")
                break
            elif opcode_name.startswith("jump"):
                # Could analyze jump target
                print("    Jump instruction detected")
                
            pc = next_pc
            print()

    def analyze_memory_regions(self):
        """Analyze different memory regions"""
        print("\nMemory Region Analysis:")
        print("=" * 40)
        
        print(f"Code region: 0x0000 - 0x{self.static_memory:04x}")
        print(f"Static region: 0x{self.static_memory:04x} - 0x{self.high_memory:04x}")  
        print(f"High memory region: 0x{self.high_memory:04x} - 0x{len(self.data):04x}")
        
        # Check for suspicious patterns
        print(f"\nChecking for patterns...")
        null_count = self.data.count(0)
        print(f"Null bytes: {null_count} ({100*null_count/len(self.data):.1f}%)")
        
        # Look for instruction-like patterns in suspicious areas
        suspicious_areas = [0xf00, 0xf08, 0x9e9]  # From the error messages
        for addr in suspicious_areas:
            if addr < len(self.data):
                print(f"Bytes at 0x{addr:04x}: {' '.join(f'{self.get_byte(addr+i):02x}' for i in range(8))}")

def main():
    if len(sys.argv) != 2:
        print("Usage: python debug_execution_trace.py <zcode_file>")
        sys.exit(1)
        
    filename = sys.argv[1]
    analyzer = ZMachineAnalyzer(filename)
    analyzer.analyze_memory_regions()
    analyzer.trace_from_start()

if __name__ == "__main__":
    main()