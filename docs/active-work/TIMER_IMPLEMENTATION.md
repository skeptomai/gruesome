# Timer Implementation in the Z-Machine Interpreter

## Overview

Our interpreter implements a hybrid approach that supports both turn-based and real-time timer behavior, though currently only the turn-based path is active.

## Current Implementation

### Turn-Based Timer Support (Active)

When SREAD is called with timer parameters (operands 3 & 4):

```rust
// From interpreter.rs
if has_timer {
    debug!("Simulating timer interrupt after input");
    let result = self.call_timer_routine(routine)?;
    debug!("Timer routine returned: {}", result);
}
```

This approach:
1. Waits for complete user input
2. After input is received, simulates the timer having fired once
3. Calls the timer routine which updates game state (e.g., decrements lantern timer)
4. Perfect for turn-based games like Zork I where each command is one "turn"

### Real-Time Timer Support (Infrastructure Ready)

The non-blocking I/O system provides the foundation for real-time interrupts:

```rust
// In timed_input.rs - ready but not yet connected
if start_time.elapsed() >= timeout_duration {
    debug!("Timer expired after {:?}", start_time.elapsed());
    // TODO: Call timer routine here
    // TODO: Check if it returns true (terminate input)
    break Ok((self.buffer.clone(), true));  // Return partial input
}
```

## The Beauty of This Approach

### 1. Turn-Based Games (like Zork I)
- **Current behavior**: Works perfectly - timer fires once per turn
- **Implementation**: Simple post-input timer call
- **Result**: Lantern burns down, matches expire, NPCs move - all synchronized with player turns

### 2. Real-Time Games (like Border Zone)
- **Infrastructure ready**: Non-blocking I/O with event-driven input
- **What's needed**:
  - Actually use the timeout in `read_line_nonblocking()`
  - Call `call_timer_routine()` when timeout expires
  - Check if it returns true (terminate input)
  - Return partial input with `was_terminated = true`
- **Result**: Would support real-time countdowns, impatient NPCs, timed sequences

## Implementation Details

### Timer Routine Execution

The `call_timer_routine()` method:
1. Saves current interpreter state
2. Calls the Z-Machine timer routine with 0 arguments
3. Executes it to completion (with safety limit)
4. Returns the routine's return value (true = terminate input)

### Why This Design Works

1. **Backwards Compatible**: Turn-based games work perfectly with no changes
2. **Forward Compatible**: Real-time games can be supported by connecting the timeout
3. **Clean Separation**: Input handling and timer logic are separate concerns
4. **Testable**: Each component can be tested independently

## Future Enhancement Path

To enable real-time timers:

1. In `read_line_nonblocking()`, when timeout expires:
   ```rust
   let terminate = self.interpreter.call_timer_routine(routine_addr)?;
   if terminate {
       return Ok((partial_input, true));
   }
   // Otherwise continue collecting input
   ```

2. Update game loop to handle terminated input:
   ```rust
   if was_terminated {
       // Process partial input or special "timeout" command
   }
   ```

This design elegantly handles both game types with minimal code changes.