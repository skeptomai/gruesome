# Non-Blocking I/O Implementation

## Overview

The Z-Machine interpreter now uses **true non-blocking I/O** for all terminal input. This is implemented using crossterm's event system, which leverages OS-level event notification mechanisms:

- **Linux**: epoll
- **macOS**: kqueue  
- **Windows**: I/O Completion Ports (IOCP)

## Key Features

1. **No Polling**: The implementation uses `event::poll()` which blocks until an event arrives or timeout expires. The OS wakes the thread when input is available - this is NOT busy-waiting or polling.

2. **Character-by-Character Input**: Raw mode allows immediate character processing without waiting for Enter.

3. **Timer Support**: Input can be interrupted by timers (though timer routines are not yet called).

4. **Full Line Editing**: Arrow keys, backspace, and cursor movement work correctly.

5. **Paste Support**: Multi-character paste events are handled properly.

## Implementation Details

### Event Loop (timed_input.rs)

```rust
// Wait for events with timeout
if event::poll(poll_timeout)? {
    // OS has notified us of an event
    match event::read()? {
        Event::Key(key) => handle_key_event(key),
        Event::Paste(text) => handle_paste(text),
        // ... other events
    }
}
```

### Key Points

- `event::poll()` is NOT a busy-wait - it's a blocking call that returns when:
  - An event is available (keyboard, mouse, resize, etc.)
  - The timeout expires
  - The call is interrupted

- The timeout is only used to periodically check if a Z-Machine timer has expired

- For piped/redirected input, standard blocking I/O is still used

## Testing

Run the test program to see non-blocking behavior:

```bash
RUST_LOG=info cargo run --bin test_timed_input
```

You'll notice:
- Characters appear as you type (no line buffering)
- Backspace and arrow keys work immediately
- Timeouts interrupt partial input
- The CPU usage remains at 0% while waiting

## Future Enhancements

1. **Timer Routine Calls**: When a timer expires, call the Z-Machine timer routine
2. **Input Termination**: If timer routine returns true, terminate input
3. **Sound Support**: Play sounds when timer routines request it
4. **Status Line Updates**: Refresh status line during input wait