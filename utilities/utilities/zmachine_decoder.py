#!/usr/bin/env python3
"""
Z-Machine Binary Decoder Utility

Utilities for decoding Z-Machine text, properties, and addresses from compiled binaries.
Used for debugging Z-Machine compiler output and verifying binary structure.
"""

import sys
import struct


def decode_zmachine_text(data_bytes):
    """
    Decode Z-Machine encoded text from bytes.

    Args:
        data_bytes: List or bytes of Z-Machine encoded text

    Returns:
        str: Decoded text string
    """
    alphabet = 'abcdefghijklmnopqrstuvwxyz'
    text = ''

    # Process data in 2-byte words
    for i in range(0, len(data_bytes), 2):
        if i + 1 >= len(data_bytes):
            break

        word = (data_bytes[i] << 8) | data_bytes[i+1]

        # Extract 3 characters from the word (5 bits each)
        c1 = (word >> 10) & 0x1f
        c2 = (word >> 5) & 0x1f
        c3 = word & 0x1f

        for c in [c1, c2, c3]:
            if c == 0:
                text += ' '
            elif c >= 6 and c <= 31:
                text += alphabet[c-6]
            elif c == 4:  # Shift to capitals (simplified)
                continue
            elif c == 5:  # Shift to punctuation (simplified)
                continue

        # Check for end-of-string marker (bit 15 set)
        if word & 0x8000:
            break

    return text.strip()


def packed_to_unpacked_address(packed_addr):
    """
    Convert Z-Machine packed address to unpacked address.

    Args:
        packed_addr: Packed address (16-bit)

    Returns:
        int: Unpacked address (multiply by 2 for V3)
    """
    return packed_addr * 2


def decode_property_table_header(data_bytes, offset=0):
    """
    Decode Z-Machine property table header (object name).

    Args:
        data_bytes: Binary data containing property table
        offset: Offset to start of property table

    Returns:
        tuple: (name_length, decoded_name, properties_start_offset)
    """
    if offset >= len(data_bytes):
        return None, None, offset

    name_length = data_bytes[offset]

    if name_length == 0:
        return 0, "", offset + 1

    # Name is name_length * 2 bytes
    name_bytes = data_bytes[offset + 1:offset + 1 + (name_length * 2)]
    decoded_name = decode_zmachine_text(name_bytes)

    properties_start = offset + 1 + (name_length * 2)

    return name_length, decoded_name, properties_start


def decode_property_entry(data_bytes, offset):
    """
    Decode a single Z-Machine property entry.

    Args:
        data_bytes: Binary data containing property
        offset: Offset to property entry

    Returns:
        tuple: (property_number, property_size, property_data, next_offset)
    """
    if offset >= len(data_bytes):
        return None, None, None, offset

    size_byte = data_bytes[offset]

    # Property number is low 5 bits
    prop_number = size_byte & 0x1F

    # Property size calculation
    if size_byte & 0x80:  # Extended property format (V4+)
        prop_size = size_byte & 0x3F
        if prop_size == 0:
            prop_size = 64
        data_start = offset + 2
    else:  # Standard property format (V3)
        prop_size = ((size_byte >> 5) & 0x7) + 1
        data_start = offset + 1

    # Extract property data
    property_data = data_bytes[data_start:data_start + prop_size]

    return prop_number, prop_size, property_data, data_start + prop_size


def analyze_property_table(binary_file, table_offset):
    """
    Analyze complete Z-Machine property table at given offset.

    Args:
        binary_file: Path to Z-Machine binary file
        table_offset: Offset to property table in file

    Returns:
        dict: Analysis results with object name and properties
    """
    try:
        with open(binary_file, 'rb') as f:
            f.seek(table_offset)
            data = f.read(200)  # Read enough data for analysis

        # Decode object name
        name_length, object_name, props_start = decode_property_table_header(data, 0)

        results = {
            'object_name': object_name,
            'name_length': name_length,
            'properties': []
        }

        # Decode properties
        offset = props_start
        while offset < len(data):
            prop_num, prop_size, prop_data, next_offset = decode_property_entry(data, offset)

            if prop_num is None or prop_num == 0:
                break

            # Convert property data to hex string and integer value
            prop_hex = prop_data.hex() if prop_data else ""
            prop_value = None
            if len(prop_data) == 1:
                prop_value = prop_data[0]
            elif len(prop_data) == 2:
                prop_value = (prop_data[0] << 8) | prop_data[1]

            results['properties'].append({
                'number': prop_num,
                'size': prop_size,
                'data_hex': prop_hex,
                'data_value': prop_value,
                'offset': table_offset + offset
            })

            offset = next_offset

        return results

    except Exception as e:
        return {'error': str(e)}


def find_string_at_address(binary_file, address):
    """
    Decode Z-Machine string at given address.

    Args:
        binary_file: Path to Z-Machine binary file
        address: Address of string in file

    Returns:
        str: Decoded string
    """
    try:
        with open(binary_file, 'rb') as f:
            f.seek(address)
            # Read up to 200 bytes (should be enough for most strings)
            data = f.read(200)

        return decode_zmachine_text(data)

    except Exception as e:
        return f"Error: {e}"


def main():
    """Command line interface for the decoder utilities."""
    if len(sys.argv) < 2:
        print("Usage:")
        print("  python zmachine_decoder.py decode_text <hex_bytes>")
        print("  python zmachine_decoder.py packed_addr <packed_address>")
        print("  python zmachine_decoder.py analyze_property <binary_file> <table_offset>")
        print("  python zmachine_decoder.py decode_string <binary_file> <address>")
        print("")
        print("Examples:")
        print("  python zmachine_decoder.py decode_text '138a63205160'")
        print("  python zmachine_decoder.py packed_addr 0x0608")
        print("  python zmachine_decoder.py analyze_property game.z3 0x0495")
        print("  python zmachine_decoder.py decode_string game.z3 0x0c10")
        return

    command = sys.argv[1]

    if command == "decode_text":
        if len(sys.argv) < 3:
            print("Usage: decode_text <hex_bytes>")
            return
        hex_string = sys.argv[2].replace('0x', '')
        data_bytes = bytes.fromhex(hex_string)
        result = decode_zmachine_text(data_bytes)
        print(f"Decoded text: '{result}'")

    elif command == "packed_addr":
        if len(sys.argv) < 3:
            print("Usage: packed_addr <packed_address>")
            return
        packed = int(sys.argv[2], 0)  # Auto-detect hex/decimal
        unpacked = packed_to_unpacked_address(packed)
        print(f"Packed address 0x{packed:04x} -> unpacked address 0x{unpacked:04x}")

    elif command == "analyze_property":
        if len(sys.argv) < 4:
            print("Usage: analyze_property <binary_file> <table_offset>")
            return
        binary_file = sys.argv[2]
        table_offset = int(sys.argv[3], 0)
        result = analyze_property_table(binary_file, table_offset)

        if 'error' in result:
            print(f"Error: {result['error']}")
            return

        print(f"Object: '{result['object_name']}'")
        print(f"Properties ({len(result['properties'])}):")
        for prop in result['properties']:
            print(f"  Property {prop['number']}: {prop['size']} bytes = 0x{prop['data_hex']} ({prop['data_value']}) @ 0x{prop['offset']:04x}")

    elif command == "decode_string":
        if len(sys.argv) < 4:
            print("Usage: decode_string <binary_file> <address>")
            return
        binary_file = sys.argv[2]
        address = int(sys.argv[3], 0)
        result = find_string_at_address(binary_file, address)
        print(f"String at 0x{address:04x}: '{result}'")

    else:
        print(f"Unknown command: {command}")


if __name__ == "__main__":
    main()