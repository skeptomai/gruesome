# TXD OVERZEALOUS SCANNING ANALYSIS (November 13, 2025)

## CRITICAL FINDING: OUR SCANNING WAS OVERZEALOUS ⚠️

**USER WAS CORRECT** - We were incorrectly flagging every 16-bit value as a potential packed address violation.

## EVIDENCE

### **Commercial Zork I Behavior**
- **Our scan**: 4,483 "violations" in commercial Zork I
- **TXD result**: Runs successfully, generates 50MB output file
- **Conclusion**: Most 16-bit values are NOT treated as packed addresses by TXD

### **Context-Sensitive Address Interpretation**
TXD only interprets 16-bit values as packed addresses in specific contexts:

1. **Routine call operands** (CALL instructions)
2. **String operands** (PRINT_PADDR instructions)
3. **Property table entries** (when scanning object properties)
4. **Grammar table entries** (when parsing action handlers)
5. **Specific data structure fields** (not arbitrary data)

### **Invalid Scan Approach**
Our scan checked **every 16-bit value** in the file:
```rust
for addr in (0..memory.len()-1).step_by(2) {
    let packed = ((memory[addr] as u16) << 8) | (memory[addr + 1] as u16);
    // This treats EVERYTHING as a potential packed address!
}
```

**Problems:**
- String content like `"250905"` (serial number) treated as addresses
- Dictionary Z-character data treated as addresses
- Random game data treated as addresses
- Instruction operands treated as routine addresses

## CORRECT APPROACH

TXD failure on our file means **a specific context is generating an invalid packed address**:

1. **Instruction operand**: A CALL/PRINT_PADDR instruction has invalid address `0x6468`
2. **Data structure**: Object/property/grammar table contains invalid reference
3. **Systematic pattern**: Our compiler generates invalid addresses in specific contexts

## NEXT STEPS

1. **Remove overzealous scanning** - don't check every 16-bit value
2. **Context-specific scanning** - only check actual packed address usage contexts
3. **Trace TXD failure** - identify which specific instruction/data structure uses `0x6468`
4. **Fix the specific generator** - correct the compiler component creating that address

## LESSON LEARNED

**Z-Machine compliance is context-sensitive** - the same bit pattern can be valid data in one context and an invalid address in another. TXD's intelligence lies in knowing which contexts to treat as packed addresses.