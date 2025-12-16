# read_char Implementation

## Overview

The `read_char` opcode (VAR:0x16) is implemented with full timer support for V4+ games. This opcode reads a single character from the keyboard with optional timeout.

## Opcode Details

**Format**: VAR:0x16 (Variable form, opcode 0x16)  
**Version**: V4+ only  
**Store**: Yes (character code or 0 on timeout)  
**Operands**:
1. `keyboard` - Must be 1 (read from keyboard)
2. `time` (optional) - Timeout in tenths of seconds
3. `routine` (optional) - Timer routine address

## Implementation

### Character Code Mapping

The implementation returns standard Z-Machine character codes:
- Normal characters: ASCII value
- Return/Enter: 13
- Backspace/Delete: 8
- Escape: 27
- Timeout: 0

### Timer Support

When called with timer parameters:
1. Sets up non-blocking character input
2. Calls timer routine when timeout expires
3. If timer routine returns true, terminates input and returns 0
4. If timer routine returns false, continues reading and resets timer

### Architecture

```
Interpreter (read_char opcode)
    |
    v
read_single_char() with callback
    |
    v
TimedInput::read_char_with_timeout_callback()
    |
    +-- Piped input: read_char_standard() [blocking]
    |
    +-- Terminal input: read_char_nonblocking() [event-driven]
            |
            +-- crossterm event system
            +-- Timer callback on timeout
            +-- Raw mode for immediate response
```

## Usage Examples

### Basic Character Read (no timeout)
```
@read_char 1 -> char;
```

### Character Read with Timeout
```
@read_char 1 50 timer_routine -> char;  ; 5 second timeout
@je char 0 ?timeout;                     ; Check if timed out
```

### Games That Use read_char

- **Border Zone** (V5) - For real-time spy sequences
- **Bureaucracy** (V4) - For form filling with timeouts
- **Beyond Zork** (V5) - For combat and timed responses
- **The Lurking Horror** (V4) - For chase sequences

## Testing

Run the test program:
```bash
cargo run --bin test_read_char
```

This tests:
1. Basic character reading
2. Character reading with non-terminating timer
3. Character reading with terminating timer

## Notes

- Zork I (V3) does NOT support read_char
- The implementation uses raw terminal mode for immediate response
- Timer callbacks can fire multiple times if they return false
- Special keys (arrows, function keys) are currently ignored