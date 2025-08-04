# Prompt for Claude to create a Z Machine Disassembler

You are an accomplished C and Rust developer. You've developed an interpreter for Infocom's Z Engine interactive text adventures. The source code is in our working directory here and more importantly the specification is in @../Z-Engine-Standard in HTML form. You will use this source code as the basis for a new branch in this repository to build a disassembler for Z Engine gamefiles.  However, this source code will be informed by a full working disassembler that already exists, in C, in @../ztools/txd.c.

Here are some rules to follow:

1. Refer to the Z machine specification, especially for topics like opcode definition, program counter start, base high memory versus initial program counter (PC), and how routines can start only on even addresses, at least for v3.
2. Deeply analyze, instrument, and understand the working assembler, txd.c, written in C, before proceeding. Understand how it discovers routines by start address, and disassembles until finding a true return opcode. This is mandatory.
3. Routines to be disassembled may be the result of CALL instructions, or maybe sound, input or interrupt routines not directly callable. Those are harder to directly find. Remember the rule about routines starting at packed even addresses.  
4. Find the end of routines with actual opcode analysis, not estimate. All routines end with some form of return instruction. Follow the spec for this.
5. Follow txd.c process of serial discovery, validation and filtering by stages.  In each stage, emit debug output.
6. You may rebuild txd itself by running make, and may add additional debugging statements there.
7. Your output must match txd's output and you must use txd's output for direct comparison, iteratively, as you develop.  Always verify your outputs against txd before continuing. 
8. Analyze and implement txd's exact boundary scanning logic, complete with multi-pass and boundary expansion based on finding addresses based on call opcodes from high mem start to initial pc. Verify by your own analysis that this is what txd is doing. implement TXD's exact two-phase algorithm:
  1. Phase 1: Scan code_base to initial_pc (conservative)
  2. Phase 2: Only scan addresses discovered through call analysis
where TXD:
  1. Finds call targets in discovered routines
  2. Expands boundaries to include those targets
  3. Scans the new boundary areas
  4. Repeats until no new boundaries
Looking at TXD's exact algorithm:
  TXD doesn't just
  scan a fixed range - it has an iterative expansion algorithm that:
     1. Starts with boundaries at initial_pc
     2. Scans within current boundaries to find operands
     3. When operands point outside boundaries but have valid headers, expands the boundaries
     4. Repeats until no more boundary expansions happen
     5. Then does a final high routine scan 

TXD's Actual Scanning Algorithm:

  1. Initial boundary setup (lines 444-445): Start with low_address = code_base (4e38) and high_address = code_base (4e38)
  2. Main scanning loop (line 466): while (decode.pc <= high_pc || decode.pc <= decode.initial_pc)
    - This means scan from 4e38 up to AT LEAST initial_pc (4f05)
    - But as boundaries expand, scan further
  3. Boundary expansion: As operands are found that point outside current boundaries, expand to include them

  So TXD does indeed scan the entire range from high memory start (4e38) to initial_pc (4f05) as the base range, then expands beyond that through operand analysis.

  This explains why:
  - TXD starts boundaries at 4e38 (code_base)
  - TXD scans through the range 4e38→4f05 finding many routines
  - TXD then expands boundaries when it finds operands like 0x90ba, 0x9530, etc.

TXD's Validation Logic

  1. Basic Header Validation (lines 560-562):
  vars = read_data_byte(&decode.pc);
  if (vars >= 0 && vars <= 15)  // Must have valid variable count

  2. Instruction Decoding Validation (line 574):
  if (decode_code() == END_OF_ROUTINE) {
      // SUCCESS - valid routine
  } else {
      // FAILURE - invalid routine
  }

  3. Return Instruction Detection (lines 1646-1648):
  if (opcode.type == RETURN)
      if (decode.pc > decode.high_pc)
          return (END_OF_ROUTINE);

  4. Valid Return Instructions (lines 987, 988, 947, 948, 990, 994, 996):
  - RTRUE (0x00)
  - RFALSE (0x01)
  - RET (0x0B)
  - JUMP (0x0C)
  - PRINT_RET (0x03)
  - RET_POPPED (0x08)
  - QUIT (0x0A)

  Key Insight: TXD validates by attempting to decode instructions sequentially until it finds a RETURN-type instruction. If decoding fails at any point OR no return is found,
  the routine is invalid.

Revised Understanding of TXD's Validation

  TXD's validation works like this:

  1. Set decode.high_pc = decode.pc at start (line 686)
  2. Track maximum PC during decoding - decode.high_pc gets updated to the highest PC reached (lines 1112-1113, 1446-1447, 1630-1631)
  3. RETURN validation requires decode.pc > decode.high_pc (line 1647) - this is only true when PC advances beyond previous maximum

  The key insight: decode.high_pc tracks the MAXIMUM PC reached during the entire decoding process, not just the starting PC. The condition decode.pc > decode.high_pc ensures
  we've reached a new maximum PC position when hitting a RETURN.


1.  Your debug statements in the rust version must use log debug not eprintln
2.  You may read, copy, list, wc, cargo run, make, cc without asking for additional permission to continue
3.  For long or deep edits, ask before making the code edit. We'll often comment and commit before continuing so that we can revert and rollback if we get too deep in errors or off track algorithmically.
4.  No cheerleading, no sycophancy, no "we made great progress" until we are 100% complete. Nothing in between counts.
5.  You may run bash commands including cd to other directories, comm, echo, cargo run, cc, and make without further permission. Ask my permission before committing to git.

Start by developing a design plan, create a list of top level tasks, and present those to me before beginning work.


