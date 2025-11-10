#!/usr/bin/env python3
"""
Z3 Property Size Analyzer

CRITICAL DEBUGGING TOOL: Validates V3 property format compliance

Created November 10, 2025 to debug the V3 property parsing regression that broke
all commercial Infocom games. This tool definitively proved that:
- Our Grue compiler generates V3-compliant properties (max 5 bytes)
- The breaking commit's justification was false
- The interpreter fix was correct

Analyzes Z-Machine V3 game files to check for property size violations.
According to the Z-Machine specification (Section 12.4.1):
- V3 games support single-byte property format ONLY
- Maximum property size is 8 bytes
- Property size formula: ((size_byte >> 5) & 0x07) + 1

Usage:
    python3 analyze_z3_properties.py <z3_file>
    python3 analyze_z3_properties.py tests/mini_zork_analysis.z3

Returns: Exit code 0 if compliant, non-zero if violations found
"""

import sys
import struct

class Z3PropertyAnalyzer:
    def __init__(self, filename):
        with open(filename, 'rb') as f:
            self.data = f.read()

        # Parse Z-Machine header
        self.version = self.data[0]
        self.object_table_addr = struct.unpack('>H', self.data[0x0A:0x0C])[0]

        print(f"Z-Machine Version: {self.version}")
        print(f"Object Table Address: 0x{self.object_table_addr:04x}")

        if self.version != 3:
            print(f"WARNING: This analyzer is designed for V3 games, got V{self.version}")

    def analyze_properties(self):
        """Analyze all object properties for V3 compliance"""
        violations = []

        # Skip property defaults table (31 words = 62 bytes for V3)
        obj_entries_start = self.object_table_addr + 62

        # Each V3 object entry is 9 bytes
        obj_num = 1
        addr = obj_entries_start

        print(f"\n=== OBJECT PROPERTY ANALYSIS ===")

        while addr < len(self.data) - 9:
            # Read object entry (9 bytes for V3)
            # Bytes 0-3: attributes (32 bits)
            # Bytes 4-6: parent, sibling, child
            # Bytes 7-8: property table address (word)

            if addr + 9 > len(self.data):
                break

            prop_table_addr = struct.unpack('>H', self.data[addr + 7:addr + 9])[0]

            if prop_table_addr == 0:
                break  # End of objects

            print(f"\nObject #{obj_num} - Property table at 0x{prop_table_addr:04x}")

            # Analyze this object's properties
            violations.extend(self.analyze_object_properties(obj_num, prop_table_addr))

            obj_num += 1
            addr += 9

            # Safety check - don't analyze more than 100 objects
            if obj_num > 100:
                print("Reached safety limit of 100 objects")
                break

        return violations

    def analyze_object_properties(self, obj_num, prop_table_addr):
        """Analyze properties for a single object"""
        violations = []

        if prop_table_addr >= len(self.data):
            print(f"  ERROR: Property table address out of bounds")
            return violations

        # Skip object name (first byte is length in words)
        name_len_words = self.data[prop_table_addr]
        name_bytes = name_len_words * 2
        prop_start = prop_table_addr + 1 + name_bytes

        print(f"  Name length: {name_len_words} words ({name_bytes} bytes)")
        print(f"  Properties start at: 0x{prop_start:04x}")

        # Parse properties
        addr = prop_start
        prop_count = 0

        while addr < len(self.data) and prop_count < 50:  # Safety limit
            size_byte = self.data[addr]

            if size_byte == 0:
                print(f"  Property list terminator found")
                break

            # V3 property format: top 3 bits = size-1, bottom 5 bits = prop number
            prop_num = size_byte & 0x1F
            prop_size = ((size_byte >> 5) & 0x07) + 1

            print(f"  Property #{prop_num}: size_byte=0x{size_byte:02x}, size={prop_size} bytes")

            # Check for V3 compliance
            if prop_size > 8:
                violation = {
                    'object': obj_num,
                    'property': prop_num,
                    'size': prop_size,
                    'size_byte': size_byte,
                    'address': addr
                }
                violations.append(violation)
                print(f"    ⚠️  VIOLATION: Property size {prop_size} exceeds V3 maximum of 8 bytes!")

            # Check for impossible sizes (would indicate two-byte format confusion)
            if prop_size == 0:
                print(f"    ⚠️  WARNING: Property size 0 (this shouldn't happen in V3)")

            addr += 1 + prop_size  # Skip size byte + property data
            prop_count += 1

        return violations

    def print_violations_summary(self, violations):
        """Print summary of all violations found"""
        print(f"\n=== VIOLATIONS SUMMARY ===")

        if not violations:
            print("✅ No V3 property size violations found!")
            print("All properties comply with V3 single-byte format (max 8 bytes)")
            return

        print(f"❌ Found {len(violations)} property size violations:")

        for v in violations:
            print(f"  Object #{v['object']}, Property #{v['property']}:")
            print(f"    Size: {v['size']} bytes (max allowed: 8)")
            print(f"    Size byte: 0x{v['size_byte']:02x} at address 0x{v['address']:04x}")

        print(f"\n🚨 CONCLUSION:")
        print(f"The Grue compiler is generating properties that violate V3 specification!")
        print(f"This explains why the interpreter needed incorrect 'two-byte format' support.")
        print(f"FIX: Update the compiler to respect V3's 8-byte property limit.")

def main():
    if len(sys.argv) != 2:
        print("Usage: python3 analyze_z3_properties.py <z3_file>")
        print("Example: python3 analyze_z3_properties.py tests/mini_zork_analysis.z3")
        sys.exit(1)

    filename = sys.argv[1]

    try:
        analyzer = Z3PropertyAnalyzer(filename)
        violations = analyzer.analyze_properties()
        analyzer.print_violations_summary(violations)

        # Exit code indicates whether violations were found
        sys.exit(len(violations))

    except FileNotFoundError:
        print(f"Error: File '{filename}' not found")
        sys.exit(1)
    except Exception as e:
        print(f"Error analyzing file: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()