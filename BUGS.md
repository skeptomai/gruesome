# List of outstanding bugs

## FIXED - Nov 7, 2025

* ✅ **COMPLETELY FIXED**: Blocked exit messages printing "0" instead of message text
  - **Issue**: `exit_get_message` builtin used push/pull mechanism which failed, causing `println(exit.message)` to print "0"
  - **Root Cause**: Two-part issue:
    1. `exit_get_message` used push/pull mechanism vs direct storage
    2. `println()` didn't know string addresses should use `print_paddr` instead of `print_num`
  - **Complete Fix**:
    1. Updated `exit_get_message` to use direct storage like `exit_get_destination`
    2. Created new `print_message()` builtin that uses `print_paddr` for string addresses
    3. Updated mini_zork.grue to use `print_message(exit.message)` instead of `println(exit.message)`
  - **Files Changed**:
    - `src/grue_compiler/codegen_builtins.rs` (lines 1467-1479, 1500-1530)
    - `src/grue_compiler/codegen.rs` (line 5849)
    - `src/grue_compiler/semantic.rs` (line 87)
    - `src/grue_compiler/ir.rs` (lines 1073, 1128)
    - `examples/mini_zork.grue` (line 388)
  - **Additional Fix**: Moved `result_var` allocation in `get_exit` function to proper scope for both "found" and "not found" paths

**Original Issue**:
```
You are standing in an open field west of a white house, with a boarded front door.
There is a small mailbox here.
> east
0> quit
Are you sure you want to quit? (y/n)
```

**Expected Result**:
```
> east
The door is boarded and you can't remove the boards.
```