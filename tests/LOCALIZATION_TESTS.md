# Localization System Regression Tests

This directory contains comprehensive test files for validating the localization message system. These tests were created during Phase 6 implementation and should be run for regression testing.

## Test Files Overview

### Core Functionality Tests

#### `comprehensive_localization_test.grue`
**Purpose**: Complete localization system validation
**Features**: Custom prompt, custom error messages, builtin placeholders
**Expected Behavior**:
- Prompt: `"Enter command: "`
- Error: `"Sorry, I don't recognize that command. Try 'help' for assistance."`
- All custom messages working in real gameplay

#### `fallback_test.grue`
**Purpose**: Validates fallback behavior when no messages block is defined
**Features**: No messages{} block - should use defaults
**Expected Behavior**:
- Prompt: `"> "` (classic Z-Machine style)
- Error: `"I don't understand that."` (original default)

### International/Localization Examples

#### `french_localization_test.grue`
**Purpose**: Demonstrates French localization capability
**Features**: French system messages, French game content
**Expected Behavior**:
- Prompt: `"Commande: "`
- Error: `"Désolé, je ne comprends pas cette commande. Tapez 'aide' pour assistance."`

#### `spanish_localization_test.grue`
**Purpose**: Demonstrates Spanish localization capability
**Features**: Spanish system messages, Spanish game content
**Expected Behavior**:
- Prompt: `"Comando: "`
- Error: `"Lo siento, no entiendo ese comando. Escriba 'ayuda' para asistencia."`

### Phase 5 Builtin Integration Tests

#### `test_phase5_builtin_messages.grue`
**Purpose**: Tests builtin function message integration
**Features**: Custom builtin placeholder messages
**Expected Behavior**: Builtin functions use custom messages when available

#### `test_phase5_no_messages.grue`
**Purpose**: Tests Phase 5 without custom messages
**Features**: No messages block for Phase 5 comparison
**Expected Behavior**: Builtin functions use default fallback messages

## Running Regression Tests

### Compile All Tests
```bash
# Test compilation of all localization test files
for file in tests/comprehensive_localization_test.grue tests/fallback_test.grue tests/french_localization_test.grue tests/spanish_localization_test.grue tests/test_phase5_builtin_messages.grue tests/test_phase5_no_messages.grue; do
    echo "Compiling $file..."
    RUST_LOG=off cargo run --bin grue-compiler -- "$file" -o "${file%.grue}.z3"
done
```

### Test Custom Messages
```bash
# Test that custom messages work
echo -e "look\ninvalid_command\nquit\ny" | ./target/debug/gruesome tests/comprehensive_localization_test.z3
# Should show: "Enter command:" and custom error message
```

### Test Fallback Behavior
```bash
# Test that fallback messages work when no custom messages defined
echo -e "look\ninvalid_command\nquit\ny" | ./target/debug/gruesome tests/fallback_test.z3
# Should show: "> " and "I don't understand that."
```

### Test International Support
```bash
# Test French localization
echo -e "regarder\ncommande_invalide\nquit\ny" | ./target/debug/gruesome tests/french_localization_test.z3
# Should show French prompt and error messages

# Test Spanish localization
echo -e "mirar\ncomando_invalido\nquit\ny" | ./target/debug/gruesome tests/spanish_localization_test.z3
# Should show Spanish prompt and error messages
```

## Validation Checklist

For each regression test run, verify:

- ✅ **Compilation Success**: All .grue files compile without errors
- ✅ **Custom Messages**: Custom prompt and error messages display correctly
- ✅ **Fallback Behavior**: Default messages used when no custom messages defined
- ✅ **Backward Compatibility**: Existing games (like mini_zork) continue to work unchanged
- ✅ **International Support**: Non-English messages work correctly
- ✅ **No Regressions**: All existing functionality preserved

## Test Results Archive

These tests were validated during Phase 6 implementation on 2025-11-10:
- ✅ All tests passed real gameplay validation
- ✅ Custom and fallback messages working perfectly
- ✅ International localization demonstrated successfully
- ✅ Zero regressions observed
- ✅ Backward compatibility with mini_zork confirmed

## Future Enhancements

These test files can be extended to test:
- Additional languages (German, Italian, etc.)
- Complex Unicode support (when Z-Machine encoding supports it)
- Region-specific message variants
- Dynamic message switching at runtime