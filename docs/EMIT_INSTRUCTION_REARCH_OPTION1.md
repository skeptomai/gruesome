# Option 1: Typed Opcode Enums

## Core Design

Use Rust's type system to encode instruction forms directly into opcode types:

```rust
// In src/grue_compiler/opcodes.rs (new file)

/// 0OP form instructions (no operands, encoded as SHORT form with type=11)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op0 {
    Rtrue = 0x00,      // 0OP:176 (0xB0)
    Rfalse = 0x01,     // 0OP:177 (0xB1)
    Print = 0x02,      // 0OP:178 (0xB2)
    PrintRet = 0x03,   // 0OP:179 (0xB3)
    Nop = 0x04,        // 0OP:180 (0xB4)
    Quit = 0x0A,       // 0OP:186 (0xBA)
    NewLine = 0x0B,    // 0OP:187 (0xBB)
    // ... more 0OP opcodes
}

/// 1OP form instructions (1 operand, encoded as SHORT form)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op1 {
    Jz = 0x00,         // 1OP:128 (0x80/0x90/0xA0 depending on operand type)
    GetSibling = 0x01, // 1OP:129
    GetChild = 0x02,   // 1OP:130
    GetParent = 0x03,  // 1OP:131
    GetPropLen = 0x04, // 1OP:132
    Inc = 0x05,        // 1OP:133
    Dec = 0x06,        // 1OP:134
    PrintAddr = 0x07,  // 1OP:135
    Call1s = 0x08,     // 1OP:136 (V4+)
    RemoveObj = 0x09,  // 1OP:137
    PrintObj = 0x0A,   // 1OP:138
    Ret = 0x0B,        // 1OP:139
    Jump = 0x0C,       // 1OP:140
    PrintPaddr = 0x0D, // 1OP:141
    Load = 0x0E,       // 1OP:142
    Not = 0x0F,        // 1OP:143 (V1-4) / Call1n (V5+)
}

/// 2OP form instructions (2 operands, encoded as LONG or VAR form)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op2 {
    // Branches (no store)
    Je = 0x01,         // 2OP:1
    Jl = 0x02,         // 2OP:2
    Jg = 0x03,         // 2OP:3

    // Comparisons
    DecChk = 0x04,     // 2OP:4
    IncChk = 0x05,     // 2OP:5
    Jin = 0x06,        // 2OP:6
    Test = 0x07,       // 2OP:7

    // Arithmetic (store result)
    Or = 0x08,         // 2OP:8
    And = 0x09,        // 2OP:9
    TestAttr = 0x0A,   // 2OP:10
    SetAttr = 0x0B,    // 2OP:11
    ClearAttr = 0x0C,  // 2OP:12
    Store = 0x0D,      // 2OP:13
    InsertObj = 0x0E,  // 2OP:14
    Loadw = 0x0F,      // 2OP:15
    Loadb = 0x10,      // 2OP:16
    GetProp = 0x11,    // 2OP:17
    GetPropAddr = 0x12,// 2OP:18
    GetNextProp = 0x13,// 2OP:19
    Add = 0x14,        // 2OP:20
    Sub = 0x15,        // 2OP:21
    Mul = 0x16,        // 2OP:22
    Div = 0x17,        // 2OP:23
    Mod = 0x18,        // 2OP:24
    // V4+
    Call2s = 0x19,     // 2OP:25 (V4+)
    Call2n = 0x1A,     // 2OP:26 (V5+)
    SetColour = 0x1B,  // 2OP:27 (V5+)
    Throw = 0x1C,      // 2OP:28 (V5+)
}

/// VAR form instructions (0-4 operands in V3, 0-8 in V4+)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpVar {
    CallVs = 0x00,     // VAR:224 (0xE0)
    Storew = 0x01,     // VAR:225 (0xE1)
    Storeb = 0x02,     // VAR:226 (0xE2)
    PutProp = 0x03,    // VAR:227 (0xE3)
    Sread = 0x04,      // VAR:228 (0xE4) - V1-3 / Aread V4+
    PrintChar = 0x05,  // VAR:229 (0xE5)
    PrintNum = 0x06,   // VAR:230 (0xE6)
    Random = 0x07,     // VAR:231 (0xE7)
    Push = 0x08,       // VAR:232 (0xE8)
    Pull = 0x09,       // VAR:233 (0xE9) - V1-5 / Pull stack V6
    // V3+
    SplitWindow = 0x0A,// VAR:234 (0xEA) V3+
    SetWindow = 0x0B,  // VAR:235 (0xEB) V3+
    // V4+
    CallVs2 = 0x0C,    // VAR:236 (0xEC) V4+
    EraseWindow = 0x0D,// VAR:237 (0xED) V4+
    EraseLine = 0x0E,  // VAR:238 (0xEE) V4+
    SetCursor = 0x0F,  // VAR:239 (0xEF) V4+
    GetCursor = 0x10,  // VAR:240 (0xF0) V4+
    SetTextStyle = 0x11,// VAR:241 (0xF1) V4+
    BufferMode = 0x12, // VAR:242 (0xF2) V4+
    OutputStream = 0x13,// VAR:243 (0xF3) V3+
    InputStream = 0x14,// VAR:244 (0xF4) V3+
    SoundEffect = 0x15,// VAR:245 (0xF5) V3+
    // V4+
    ReadChar = 0x16,   // VAR:246 (0xF6) V4+
    ScanTable = 0x17,  // VAR:247 (0xF7) V4+
    // V5+
    Not = 0x18,        // VAR:248 (0xF8) V5+
    CallVn = 0x19,     // VAR:249 (0xF9) V5+
    CallVn2 = 0x1A,    // VAR:250 (0xFA) V5+
    Tokenise = 0x1B,   // VAR:251 (0xFB) V5+
    EncodeText = 0x1C, // VAR:252 (0xFC) V5+
    CopyTable = 0x1D,  // VAR:253 (0xFD) V5+
    PrintTable = 0x1E, // VAR:254 (0xFE) V5+
    CheckArgCount = 0x1F,// VAR:255 (0xFF) V5+
}

/// Combined opcode enum that can represent any instruction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    Op0(Op0),
    Op1(Op1),
    Op2(Op2),
    OpVar(OpVar),
}

impl Opcode {
    /// Get the raw opcode number
    pub fn raw_value(&self) -> u8 {
        match self {
            Opcode::Op0(op) => *op as u8,
            Opcode::Op1(op) => *op as u8,
            Opcode::Op2(op) => *op as u8,
            Opcode::OpVar(op) => *op as u8,
        }
    }

    /// Get the instruction form
    pub fn form(&self) -> InstructionForm {
        match self {
            Opcode::Op0(_) => InstructionForm::Short,
            Opcode::Op1(_) => InstructionForm::Short,
            Opcode::Op2(_) => InstructionForm::Long, // or Variable if needed
            Opcode::OpVar(_) => InstructionForm::Variable,
        }
    }

    /// Does this instruction store a result?
    pub fn stores_result(&self) -> bool {
        match self {
            Opcode::Op0(op) => matches!(op, Op0::Rtrue | Op0::Rfalse), // These implicitly store
            Opcode::Op1(op) => matches!(op,
                Op1::GetSibling | Op1::GetChild | Op1::GetParent |
                Op1::GetPropLen | Op1::Call1s | Op1::Load | Op1::Not
            ),
            Opcode::Op2(op) => matches!(op,
                Op2::Or | Op2::And | Op2::Loadw | Op2::Loadb |
                Op2::GetProp | Op2::GetPropAddr | Op2::GetNextProp |
                Op2::Add | Op2::Sub | Op2::Mul | Op2::Div | Op2::Mod |
                Op2::Call2s
            ),
            Opcode::OpVar(op) => matches!(op,
                OpVar::CallVs | OpVar::Random | OpVar::Pull | OpVar::CallVs2 |
                OpVar::ReadChar | OpVar::ScanTable | OpVar::Not
            ),
        }
    }

    /// Does this instruction branch?
    pub fn branches(&self) -> bool {
        match self {
            Opcode::Op0(_) => false,
            Opcode::Op1(op) => matches!(op, Op1::Jz | Op1::GetSibling | Op1::GetChild),
            Opcode::Op2(op) => matches!(op,
                Op2::Je | Op2::Jl | Op2::Jg | Op2::DecChk | Op2::IncChk |
                Op2::Jin | Op2::Test | Op2::TestAttr
            ),
            Opcode::OpVar(op) => matches!(op, OpVar::ScanTable),
        }
    }

    /// Get the expected operand count (None = variable)
    pub fn operand_count(&self) -> Option<usize> {
        match self {
            Opcode::Op0(_) => Some(0),
            Opcode::Op1(_) => Some(1),
            Opcode::Op2(_) => Some(2),
            Opcode::OpVar(_) => None, // Variable count
        }
    }
}
```

## Updated emit_instruction signature

```rust
// In src/grue_compiler/codegen_instructions.rs

impl CodeGen {
    /// Emits a Z-Machine instruction with type-safe opcode
    ///
    /// **Type Safety**: The opcode parameter is now an enum that encodes
    /// the instruction form into the type system. This eliminates ambiguity
    /// and catches errors at compile time.
    ///
    /// **Examples**:
    /// ```ignore
    /// // 0OP instruction
    /// self.emit_instruction(Opcode::Op0(Op0::Quit), &[], None, None)?;
    ///
    /// // 1OP instruction
    /// self.emit_instruction(
    ///     Opcode::Op1(Op1::PrintPaddr),
    ///     &[Operand::LargeConstant(0x1234)],
    ///     None,
    ///     None
    /// )?;
    ///
    /// // 2OP instruction with branch
    /// self.emit_instruction(
    ///     Opcode::Op2(Op2::Je),
    ///     &[Operand::Variable(1), Operand::SmallConstant(5)],
    ///     None,
    ///     Some(10)  // branch offset
    /// )?;
    ///
    /// // VAR instruction with store
    /// self.emit_instruction(
    ///     Opcode::OpVar(OpVar::CallVs),
    ///     &[Operand::LargeConstant(func_addr), Operand::Variable(1)],
    ///     Some(0),  // store to stack
    ///     None
    /// )?;
    /// ```
    pub fn emit_instruction(
        &mut self,
        opcode: Opcode,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<InstructionLayout, CompilerError> {
        let start_address = self.code_address;
        let raw_opcode = opcode.raw_value();

        // Validate operand count matches instruction type
        if let Some(expected) = opcode.operand_count() {
            if operands.len() != expected {
                return Err(CompilerError::CodeGenError(format!(
                    "Opcode {:?} expects {} operands, got {}",
                    opcode, expected, operands.len()
                )));
            }
        }

        // Validate store_var matches instruction capabilities
        if store_var.is_some() && !opcode.stores_result() {
            return Err(CompilerError::CodeGenError(format!(
                "Opcode {:?} does not store a result, but store_var was provided",
                opcode
            )));
        }

        // Validate branch_offset matches instruction capabilities
        if branch_offset.is_some() && !opcode.branches() {
            return Err(CompilerError::CodeGenError(format!(
                "Opcode {:?} does not branch, but branch_offset was provided",
                opcode
            )));
        }

        log::debug!(
            "EMIT_INSTR: addr=0x{:04x} opcode={:?} operands={:?} store={:?} branch={:?}",
            start_address, opcode, operands, store_var, branch_offset
        );

        // Use the opcode's intrinsic form instead of determining it
        let form = opcode.form();

        // Special case: 2OP can use either LONG or VARIABLE form
        let form = if matches!(opcode, Opcode::Op2(_)) {
            self.determine_2op_form(operands)?
        } else {
            form
        };

        let layout = match form {
            InstructionForm::Long => self.emit_long_form_with_layout(
                start_address,
                raw_opcode,
                operands,
                store_var,
                branch_offset,
            )?,
            InstructionForm::Short => self.emit_short_form_with_layout(
                start_address,
                raw_opcode,
                operands,
                store_var,
                branch_offset,
            )?,
            InstructionForm::Variable => self.emit_variable_form_with_layout(
                start_address,
                raw_opcode,
                operands,
                store_var,
                branch_offset,
            )?,
            InstructionForm::Extended => {
                return Err(CompilerError::CodeGenError(
                    "Extended form instructions not yet supported".to_string(),
                ));
            }
        };

        Ok(layout)
    }

    /// Determine whether to use LONG or VARIABLE form for 2OP instructions
    fn determine_2op_form(&self, operands: &[Operand]) -> Result<InstructionForm, CompilerError> {
        if operands.len() != 2 {
            return Ok(InstructionForm::Variable);
        }

        // Use LONG form if both operands fit the LONG form constraints
        match (&operands[0], &operands[1]) {
            (Operand::SmallConstant(_), Operand::SmallConstant(_)) |
            (Operand::SmallConstant(_), Operand::Variable(_)) |
            (Operand::Variable(_), Operand::SmallConstant(_)) |
            (Operand::Variable(_), Operand::Variable(_)) => {
                Ok(InstructionForm::Long)
            }
            _ => Ok(InstructionForm::Variable)
        }
    }
}
```

## Usage Examples

```rust
// Before (ambiguous):
self.emit_instruction(0x0A, &[], None, None)?;  // What is 0x0A?

// After (crystal clear):
self.emit_instruction(Opcode::Op0(Op0::Quit), &[], None, None)?;

// ----

// Before (error-prone):
self.emit_instruction(0x00, &[func_addr, arg1], Some(0), None)?;  // Is this JZ or CallVs?

// After (unambiguous):
self.emit_instruction(
    Opcode::OpVar(OpVar::CallVs),
    &[func_addr, arg1],
    Some(0),
    None
)?;

// ----

// Before (hard to read):
self.emit_instruction(0x01, &[var1, const5], None, Some(offset))?;

// After (self-documenting):
self.emit_instruction(
    Opcode::Op2(Op2::Je),
    &[var1, const5],
    None,
    Some(offset)
)?;

// ----

// Compile-time error detection:
self.emit_instruction(
    Opcode::Op0(Op0::Quit),
    &[Operand::SmallConstant(1)],  // ERROR: Quit takes 0 operands
    None,
    None
)?;  // Caught at runtime with clear error message

self.emit_instruction(
    Opcode::Op0(Op0::NewLine),
    &[],
    Some(0),  // ERROR: NewLine doesn't store a result
    None
)?;  // Caught at runtime with clear error message
```

## Migration Strategy

### Phase 1: Add the enums (non-breaking)
- Add `opcodes.rs` with all the enum definitions
- Keep existing `emit_instruction(opcode: u8, ...)` working

### Phase 2: Add new method alongside old (non-breaking)
```rust
// New type-safe version
pub fn emit_instruction_typed(
    &mut self,
    opcode: Opcode,
    operands: &[Operand],
    store_var: Option<u8>,
    branch_offset: Option<i16>,
) -> Result<InstructionLayout, CompilerError> {
    // ... new implementation
}

// Old version delegates to new version
pub fn emit_instruction(
    &mut self,
    opcode: u8,
    operands: &[Operand],
    store_var: Option<u8>,
    branch_offset: Option<i16>,
) -> Result<InstructionLayout, CompilerError> {
    // Attempt to infer the opcode type (best effort)
    let typed_opcode = self.infer_opcode_type(opcode, operands)?;
    self.emit_instruction_typed(typed_opcode, operands, store_var, branch_offset)
}
```

### Phase 3: Migrate call sites incrementally
- Update one module at a time to use `Opcode` enums
- Each migration is a clear improvement with better type safety

### Phase 4: Deprecate old method
- Mark `emit_instruction(u8, ...)` as deprecated
- Eventually remove once all call sites migrated

## Benefits

✅ **Compile-time safety**: Can't confuse Je (2OP) with Storew (VAR)
✅ **Self-documenting**: `Opcode::Op1(Op1::PrintPaddr)` is clearer than `0x0D`
✅ **IDE support**: Autocomplete shows all available opcodes for each form
✅ **Validation**: Automatic checking of operand counts, store, and branch parameters
✅ **Refactor-friendly**: Renaming/reorganizing opcodes is safe with compiler help
✅ **Spec alignment**: Enum structure mirrors Z-Machine specification

## Drawbacks

⚠️ **More verbose**: `Opcode::Op2(Op2::Add)` vs `0x14`
⚠️ **Migration effort**: ~200+ call sites need updating
⚠️ **Pattern matching**: Need to destructure enum for raw value
⚠️ **Learning curve**: Developers need to learn the enum structure

## Hybrid Approach (Best of Both Worlds)

Keep convenience constants for common cases:

```rust
// Re-export common opcodes at top level for ergonomics
pub use opcodes::{
    // 0OP shortcuts
    QUIT, NEW_LINE, RTRUE, RFALSE,
    // 1OP shortcuts
    PRINT_PADDR, JZ, LOAD, RET, JUMP,
    // 2OP shortcuts
    JE, JL, JG, ADD, SUB, MUL, DIV, STORE,
    // VAR shortcuts
    CALL_VS, PUT_PROP, SREAD, PRINT_CHAR,
};

// Usage becomes:
self.emit_instruction(QUIT, &[], None, None)?;
self.emit_instruction(PRINT_PADDR, &[addr], None, None)?;
self.emit_instruction(JE, &[var1, const5], None, Some(offset))?;
```

This gives type safety with ergonomics close to the raw numbers approach.
