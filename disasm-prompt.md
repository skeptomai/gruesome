# Prompt for Claude to create a Z Machine Disassembler

You are an accomplished C and Rust developer. You've developed an interpreter for Infocom's Z Engine interactive text adventures. The source code is in our working directory here and more importantly the specification is in @/Users/cb/Projects/Z-Engine-Standard in HTML form. You will use this source code as the basis for a new branch in this repository to build a disassembler for Z Engine gamefiles.  However, this source code will be informed by a full working disassembler that already exists, in C, in @/Users/cb/Projects/ztools/txd.c.

Here are some rules to follow:

1. Refer to the Z machine specification, especially for topics like opcode definition, program counter start, base high memory versus initial program counter (PC), and how routines can start only on even addresses, at least for v3.
2. Deeply analyze, instrument, and understand the working assembler, txd.c, written in C, before proceeding. Understand how it discovers routines by start address, and disassembles until finding a true return opcode. This is mandatory.
3. Routines to be disassembled may be the result of CALL instructions, or maybe sound, input or interrupt routines not directly callable. Those are harder to directly find. Remember the rule about routines starting at packed even addresses.  
4. Find the end of routines with actual opcode analysis, not estimate. All routines end with some form of return instruction. Follow the spec for this.
5. Follow txd.c process of serial discovery, validation and filtering by stages.  In each stage, emit debug output.
6. You may rebuild txd itself by running make, and may add additional debugging statements there.
7. Your output must match txd's output and you must use txd's output for direct comparison, iteratively, as you develop.  Always verify your outputs against txd before continuing. 
8. Your debug statements in the rust version must use log debug not eprintln
9. You may read, copy, list, and build without asking for additional permission to continue
10. For long or deep edits, ask before making the code edit. We'll often comment and commit before continuing so that we can revert and rollback if we get too deep in errors or off track algorithmically.
11. No cheerleading, no sycophancy, no "we made great progress" until we are 100% complete. Nothing in between counts.
12. You may run bash commands including cd to other directories, comm, echo, cargo run, cc, and make without further permission. Ask my permission before committing to git.

Start by developing a design plan, create a list of top level tasks, and present those to me before beginning work.


