# Localization Plan: String Management and Internationalization

## Overview

This document analyzes the current string handling architecture in the Grue compiler, with a focus on understanding how user-facing messages are managed and the implications for future localization efforts.

## Current String Architecture

### Error Message Handling: "I don't understand that"

The analysis of the "I don't understand that" message reveals the complete string management pipeline in the compiler:

#### 1. String Allocation Phase (`src/grue_compiler/codegen_strings.rs:137`)

```rust
// Also add the "I don't understand" string for command processing
let unknown_command_id = self.find_or_create_string_id("I don't understand that.")?;
debug!(
    "🎯 Allocated unknown command string ID: {}",
    unknown_command_id
);
```

**Key Insights:**
- Strings are allocated during the string collection phase
- Each string receives a unique ID for later reference
- The `find_or_create_string_id` method suggests string deduplication

#### 2. Storage Architecture (`src/grue_compiler/codegen.rs:259`)

```rust
pub main_loop_unknown_command_id: Option<IrId>, // ID of the "I don't understand" string (public for codegen_extensions.rs)
```

**Architecture Notes:**
- System-level strings are stored as specific fields in the codegen structure
- These are separate from user-defined strings in the Grue source code
- Public visibility allows access from codegen extensions

#### 3. Initialization Process (`src/grue_compiler/codegen_image.rs:51`)

```rust
let (prompt_id, unknown_command_id) = self.add_main_loop_strings()?;
self.main_loop_prompt_id = Some(prompt_id);
self.main_loop_unknown_command_id = Some(unknown_command_id);
```

**Process Flow:**
- Main loop strings are allocated after user strings
- Both prompt ("> ") and error message are handled together
- IDs are stored for later code generation

#### 4. Code Emission (`src/grue_compiler/codegen.rs:2311-2334`)

```rust
// Default handler: print unknown command and continue
let unknown_command_string_id = self
    .main_loop_unknown_command_id
    .expect("Main loop unknown command ID should be set during string collection");
let layout = self.emit_instruction_typed(
    Opcode::Op1(Op1::PrintPaddr), // print_paddr: print string at packed address
    &[Operand::LargeConstant(placeholder_word())], // Placeholder for string address
    None,
    None,
)?;

// Create unresolved reference for string address resolution
self.reference_context
    .unresolved_refs
    .push(UnresolvedReference {
        reference_type: LegacyReferenceType::StringRef,
        location: operand_location,
        target_id: unknown_command_string_id,
        is_packed_address: true,
        offset_size: 2,
        location_space: MemorySpace::Code,
    });
```

**Technical Implementation:**
- Uses Z-Machine `print_paddr` instruction for string output
- Placeholder system allows forward references to strings
- String addresses are resolved during final linking phase

## Runtime Execution Flow

### Complete Chain: Compilation to Console Output

**When an unrecognized command is entered (like "foobar"):**

#### 1. **Compiler Generation**
The compiler emits this instruction sequence in the main loop fallthrough case (`src/grue_compiler/codegen.rs:2311-2334`):

```rust
// Default handler: print unknown command and continue
let unknown_command_string_id = self
    .main_loop_unknown_command_id
    .expect("Main loop unknown command ID should be set during string collection");
let layout = self.emit_instruction_typed(
    Opcode::Op1(Op1::PrintPaddr), // print_paddr: print string at packed address
    &[Operand::LargeConstant(placeholder_word())], // Placeholder for string address
    None,
    None,
)?;
```

This generates a Z-Machine `print_paddr` instruction (opcode 0x8D) with the packed address of the "I don't understand that." string.

#### 2. **Runtime Pattern Matching**
During execution, the main loop:
- Reads user input into parse buffer
- Attempts to match against all verb patterns in the grammar
- When no patterns match, execution **falls through** to the default case

#### 3. **Interpreter Execution**
The Z-Machine interpreter executes the `print_paddr` instruction (`src/opcodes_display.rs:101-128`):

```rust
// 1OP:0x0D - print_paddr
(0x0D, crate::instruction::OperandCount::OP1) => {
    // Print string at packed address
    let pc = self.vm.pc - inst.size as u32;
    debug!("print_paddr at {:05x}: operand={:04x}", pc, operands[0]);

    let abbrev_addr = self.vm.game.header.abbrev_table;
    match crate::text::decode_string_at_packed_addr(
        &self.vm.game.memory,
        operands[0],
        self.vm.game.header.version,
        abbrev_addr,
    ) {
        Ok(string) => {
            self.output_text(&string)?;  // <-- String gets output here
        }
        Err(e) => {
            debug!("Failed to decode string at {:04x}: {}", operands[0], e);
        }
    }
    Ok(ExecutionResult::Continue)
}
```

#### 4. **Text Output**
Finally, `output_text()` sends the string to the console (`src/interpreter.rs:2835-2843`):

```rust
// Send to screen (whether stream 3 is active or not)
if let Some(ref mut display) = self.display {
    display.print(text).ok();
} else {
    print!("{text}");        // <-- Console output happens here
    io::stdout().flush().ok();
}
```

### Runtime Flow Summary

1. **String allocation**: "I don't understand that." allocated with unique ID during compilation
2. **Compiler emit**: `print_paddr` instruction generated in main loop fallthrough
3. **Grammar matching**: User input fails to match any verb patterns at runtime
4. **Execution falls through**: to the default `print_paddr` instruction
5. **String decoding**: Z-Machine decodes the packed string from memory
6. **Console output**: `print!("{text}")` displays "I don't understand that." to stdout

**Key Insight**: The message appears whenever the grammar pattern matching system cannot find a matching verb pattern for the user's input, causing execution to fall through to this default error handler in the compiled main loop.

## Localization Implications

### Current System Analysis

**Strengths:**
1. **Centralized String Management**: All strings go through `find_or_create_string_id`
2. **Deferred Resolution**: Placeholder system allows flexible string placement
3. **Separation of Concerns**: System strings vs. user strings are handled separately
4. **Unique Identification**: Each string has a stable ID throughout compilation

**Challenges for Localization:**
1. **Hardcoded Literals**: Error messages are embedded as string literals in Rust code
2. **System String Distribution**: Main loop strings scattered across multiple modules
3. **No Message Catalog**: No central registry of translatable strings
4. **Build-Time Binding**: Language selection would need to occur at compile time

### Identified System Strings

Based on this analysis, the following system strings require localization consideration:

1. **Command Prompt**: `"> "` (allocated in `add_main_loop_strings`)
2. **Unknown Command**: `"I don't understand that."` (allocated in `add_main_loop_strings`)
3. **Additional Messages**: Likely scattered throughout builtin function implementations

## Recommended Localization Architecture

### Phase 1: String Extraction

1. **Create Message Catalog System**
   - Define `MessageCatalog` trait for language-specific string sources
   - Implement default English catalog with all system messages
   - Create message ID constants to replace hardcoded literals

2. **Modify String Allocation**
   ```rust
   // Instead of:
   let unknown_command_id = self.find_or_create_string_id("I don't understand that.")?;

   // Use:
   let unknown_command_id = self.find_or_create_string_id(
       self.message_catalog.get_message(MessageId::UnknownCommand)?
   )?;
   ```

### Phase 2: Builtin Function Analysis

1. **Audit Builtin Functions**: Identify all user-facing messages in builtin implementations
2. **Extract Message Patterns**: Document dynamic message construction (e.g., "You can't see any such thing")
3. **Parameter Substitution**: Design system for message parameters and formatting

### Phase 3: Locale Selection

1. **Compile-Time Locale**: Add `--locale` flag to compiler
2. **Resource Loading**: Load appropriate message catalog during compilation
3. **Fallback System**: Default to English if locale unavailable

## Technical Considerations

### Z-Machine String Encoding

- Z-Machine v3 uses ZSCII character encoding
- Limited character set may restrict some languages
- String compression affects output size
- Consider character set extensions for non-Latin scripts

### Memory Constraints

- Z-Machine has strict memory limits
- String table size affects available dynamic memory
- Longer localized strings may impact game size
- Consider string compression implications

### Development Workflow

- Message extraction tools needed for translator workflow
- Build system integration for multiple locales
- Testing infrastructure for localized builds
- Documentation for translators about context and constraints

## Next Steps

1. **Audit Complete System**: Search codebase for all hardcoded user-facing strings
2. **Design Message ID System**: Create stable identifiers for all translatable messages
3. **Prototype Message Catalog**: Implement basic English catalog to validate architecture
4. **Builtin Function Review**: Analyze all builtin functions for localizable content

## Files Modified for Localization

Based on this analysis, localization will require changes to:

- `src/grue_compiler/codegen_strings.rs` - String allocation and management
- `src/grue_compiler/codegen.rs` - Main codegen structure and system string storage
- `src/grue_compiler/codegen_image.rs` - String initialization process
- `src/grue_compiler/codegen_builtins.rs` - Builtin function message handling
- Build system integration for locale selection and resource management

## Conclusion

The current string architecture provides a solid foundation for localization, with centralized string management and deferred resolution. The main challenge is extracting hardcoded literals into a message catalog system while maintaining the existing compilation pipeline's efficiency and Z-Machine compatibility.