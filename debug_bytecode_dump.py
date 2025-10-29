#!/usr/bin/env python3

import sys

def dump_bytecode(filename, start_addr, end_addr):
    with open(filename, 'rb') as f:
        data = f.read()

    print(f"Dumping bytecode from {filename} at addresses 0x{start_addr:04x} to 0x{end_addr:04x}")
    print()

    for addr in range(start_addr, min(end_addr + 1, len(data))):
        byte = data[addr]
        print(f"0x{addr:04x}: 0x{byte:02x} ({byte:08b}) '{chr(byte) if 32 <= byte <= 126 else '.'}'")

if __name__ == '__main__':
    if len(sys.argv) != 4:
        print("Usage: python debug_bytecode_dump.py <file> <start_hex> <end_hex>")
        sys.exit(1)

    filename = sys.argv[1]
    start_addr = int(sys.argv[2], 16)
    end_addr = int(sys.argv[3], 16)

    dump_bytecode(filename, start_addr, end_addr)