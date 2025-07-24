# Display Threading Architecture and Race Condition Analysis

## Problem Context
During development of the Z-Machine interpreter, we discovered that `split_window(2)` commands were being sent correctly but the display layout was still using the old value (7 lines instead of 2 lines). This caused improper window separation where the lower window appeared immediately after the status line instead of being properly separated.

## Threading Architecture

The display system uses **two separate threads**:

1. **Main Thread**: Runs the Z-Machine interpreter, executes game logic
2. **Display Thread**: Handles all terminal/screen rendering using ratatui

## Why Multiple Threads?

The display thread was created to handle:
- Non-blocking terminal input (especially for timed input like the lantern timer system)
- Terminal resize events 
- Smooth rendering without blocking game execution
- Real-time timer callbacks during user input (critical for Z-Machine v4+ games)

## The Race Condition

Here's what happens:

### Main Thread (Interpreter):
```rust
// Fast execution - sends commands rapidly via channels
self.display.erase_window(-1)?;        // Sends EraseWindow(-1) 
self.display.split_window(2)?;         // Sends SplitWindow(2)
self.display.set_window(1)?;           // Sends SetWindow(1)
// ... continues executing more commands rapidly
```

### Display Thread (Async) - OLD PROBLEMATIC CODE:
```rust
// OLD CODE - processes ONE command per 50ms timeout
loop {
    match rx.recv_timeout(Duration::from_millis(50)) {  // Wait up to 50ms for ONE command
        Ok(cmd) => {
            handle_command(&mut state, cmd)?;  // Process ONE command only
            state.render()?;                   // Render immediately with partial state
        }
        Err(timeout) => { /* handle resize events */ }
    }
}
```

## The Problem Timeline

The main thread sends commands **faster** than the display thread processes them:

```
Time 0ms:  Main thread sends EraseWindow(-1) 
Time 1ms:  Main thread sends SplitWindow(2)
Time 2ms:  Main thread sends SetWindow(1)
Time 3ms:  Display thread processes EraseWindow(-1), renders with OLD state (upper_window_lines=7)
Time 50ms: Display thread processes SplitWindow(2), renders with updated state  
Time 100ms: Display thread processes SetWindow(1)
```

**Key Issue**: Rendering happens after **each individual command**, so the layout calculation uses stale/incomplete state. The `SplitWindow(2)` command hasn't been processed yet when rendering occurs, so `state.upper_window_lines` is still 7 from previous state.

## Debug Evidence

The debug output clearly showed this race condition:

```
[DEBUG] interpreter: split_window: lines=2                    // Main thread sends command
[DEBUG] display_ratatui: split_window: 2 lines               // Main thread logs command sent  
[DEBUG] display_ratatui: Split window: created 2 lines...    // Display thread eventually processes it
[DEBUG] display_ratatui: Layout chunks: upper=Rect { height: 7 }  // But layout still uses old value!
```

The layout chunks consistently showed `height: 7` instead of `height: 2`, proving the display thread was rendering before processing the `SplitWindow(2)` command.

## The Fix - Command Queue Draining

Process **all pending commands** before rendering:

```rust
// NEW CODE - drain the entire command queue first
loop {
    let mut should_render = false;
    
    // Process ALL available commands before rendering
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(cmd) => {
                handle_command(&mut state, cmd)?;  // Process command
                should_render = true;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => break,  // No more commands, exit inner loop
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
    
    // THEN render once with fully updated state
    if should_render {
        state.render()?;
    }
}
```

## Why Not Single-Threaded?

Single-threaded would work for basic display, but breaks critical Z-Machine features:

```rust
// This blocks the entire interpreter:
let input = stdin.read_line()?;  // Blocks until user input

// Timer can't fire while blocked, so:
// - Lantern timer wouldn't count down
// - Match timer wouldn't work  
// - Real-time sequences in games like Border Zone would break
```

The Z-Machine specification requires timed interrupts during input:
- `sread` opcode accepts timer parameters (tenths of seconds)
- Timer routine must be called asynchronously during input
- Critical for turn-based counters and real-time game elements

## Z-Machine Timer Requirements

From Z-Machine specification v1.1:
- Timer specified in **tenths of seconds**
- When timer expires, `routine()` is called with no parameters
- Timer routines called asynchronously during input
- Must handle screen output from timer routines
- Games like Zork I use timers for lantern (global 88), matches (global 89), candles (global 90)

The threading architecture allows:
- Timer callbacks to execute during user input
- Non-blocking input with timeout support  
- Proper Z-Machine timer compliance
- Responsive display updates

## Conclusion

The race condition existed because:
1. Commands are sent from main thread faster than display thread processes them
2. Rendering was happening with incomplete state updates (after each command)
3. Display state (`upper_window_lines`) wasn't synchronized with command processing

The fix ensures all pending commands are processed atomically before rendering, eliminating the race condition while preserving the threading benefits needed for Z-Machine timer compliance.

## Files Modified

- `src/display_ratatui.rs`: Fixed event loop to drain command queue before rendering
- Key change in `run_display_thread()` function around lines 398-431

## Related Issues

This race condition specifically affected:
- Window separation in v4+ games like "A Mind Forever Voyaging"  
- Any rapid sequence of display commands sent from interpreter
- Layout calculations that depend on multiple commands being processed in sequence

The fix resolves the core issue where main game text appeared immediately after status line instead of in the properly separated lower window.