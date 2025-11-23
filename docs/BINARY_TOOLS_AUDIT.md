# Binary Tools Audit - Current vs Legacy

## Analysis Summary

**Total binaries**: 56
**Active/Useful**: 10-15
**Legacy debugging tools**: 40+

## Current & Useful Tools (Keep)

### Core Components ✅
- `grue_compiler.rs` - **Primary compiler binary**
- `gruedasm-txd.rs` - **Enhanced disassembler**

### Testing & Validation ✅
- `test_v3_regression.rs` - **Core functionality testing**
- `test_save_restore.rs` - **Save system validation**
- `test_timed_input.rs` - **Input system testing**
- `test_random.rs` - **RNG validation**
- `test_opcode_validation.rs` - **Z-Machine compliance**

### Game Compatibility Testing ✅
- `test_seastalker.rs` - **V3 game testing**
- `test_amfv_opening.rs` - **V4 game testing**
- `disasm_amfv.rs` - **Complex game analysis**

### Development Tools ✅
- `demo_window_ops.rs` - **V4+ window demonstrations**
- `check_addr.rs` - **Address validation utility**

## Legacy Debugging Tools (Candidates for Removal)

### Disassembler Development Artifacts (November 2025 project) 🗑️
Based on docs/DISASSEMBLER_BOUNDARY_COORDINATION_FIX.md, these were created during intensive disassembler debugging:

- `analyze_extra_routines.rs`
- `analyze_false_positives.rs`
- `analyze_nested_difference.rs`
- `analyze_routine_patterns.rs`
- `analyze_txd_heuristics.rs`
- `analyze_txd_nested.rs`
- `categorize_missing.rs`
- `check_fallthrough.rs`
- `check_nested_calls.rs`
- `check_routine_references.rs`
- `compare_routine_validity.rs`
- `debug_12a04.rs` (hardcoded AMFV analysis)
- `debug_bytecode.rs`
- `debug_call_graph.rs`
- `debug_e114_validation.rs`
- `dump_our_routines.rs`
- `filter_nested_routines.rs`
- `find_data_routines.rs`
- `fix_alternate_entries.rs`
- `implement_txd_filters.rs`
- `list_txd_routines.rs`
- `trace_false_positive.rs`
- `validate_extra_routines.rs`
- `verify_cafc_rejected.rs`
- `verify_strict_superset.rs`

### One-off Investigation Tools 🗑️
- `find_missing_13.rs` - Specific opcode investigation
- `search_read_char.rs` / `search_read_char_v4.rs` - Input research
- `demo_missing_features.rs` - Feature gap analysis
- `compare_games_summary.rs` - Game comparison utility

### Experimental/Incomplete Testing 🗑️
- `test_orphan_detection.rs` / `test_orphan_v4.rs` - Orphan routine testing
- `test_lantern_timer.rs` - Game-specific testing
- `test_dec_chk.rs` - Specific instruction testing
- `test_crossterm_coords.rs` - Terminal library testing
- `test_v4_styled_text.rs` - V4 text formatting
- `test_v4_window_selection.rs` - V4 window testing
- `test_status_overflow.rs` - Edge case testing
- `test_terminal_ops.rs` - Terminal operations
- `test_selective_status.rs` - Status line testing
- `test_amfv_part_i.rs` / `test_amfv_status.rs` - Game-specific testing
- `test_save_restore_flow.rs` - Save system flow testing
- `test_read_char.rs` - Input testing

## ✅ Implementation Complete (November 23, 2025)

**BEFORE**: 56 binaries
**AFTER**: 12 binaries (78% reduction)

### Final Binary List ✅

**Core Components**:
- `grue_compiler.rs` - Primary compiler binary
- `gruedasm-txd.rs` - Enhanced disassembler

**Essential Testing Tools**:
- `test_v3_regression.rs` - Core functionality testing
- `test_opcode_validation.rs` - Z-Machine compliance
- `test_save_restore.rs` - Save system validation
- `test_timed_input.rs` - Input system testing
- `test_random.rs` - RNG validation
- `test_seastalker.rs` - V3 game testing
- `test_amfv_opening.rs` - V4 game testing

**Development Utilities**:
- `disasm_amfv.rs` - Complex game analysis
- `demo_window_ops.rs` - V4+ window demonstrations
- `check_addr.rs` - Address validation utility

**Build Scripts**:
- `generate_opcodes.rs` - Opcode generation script

### Removed Tools (44 total) 🗑️

**Legacy disassembler analysis tools** (25 removed):
- analyze_extra_routines.rs, analyze_false_positives.rs, analyze_nested_difference.rs
- analyze_routine_patterns.rs, analyze_txd_heuristics.rs, analyze_txd_nested.rs
- categorize_missing.rs, check_fallthrough.rs, check_nested_calls.rs
- check_routine_references.rs, compare_routine_validity.rs, debug_12a04.rs
- debug_bytecode.rs, debug_call_graph.rs, debug_e114_validation.rs
- dump_our_routines.rs, filter_nested_routines.rs, find_data_routines.rs
- fix_alternate_entries.rs, implement_txd_filters.rs, list_txd_routines.rs
- trace_false_positive.rs, validate_extra_routines.rs, verify_cafc_rejected.rs
- verify_strict_superset.rs

**One-off investigation tools** (5 removed):
- find_missing_13.rs, search_read_char.rs, search_read_char_v4.rs
- demo_missing_features.rs, compare_games_summary.rs

**Redundant/experimental testing tools** (14 removed):
- test_orphan_detection.rs, test_orphan_v4.rs, test_lantern_timer.rs
- test_dec_chk.rs, test_crossterm_coords.rs, test_v4_styled_text.rs
- test_v4_window_selection.rs, test_status_overflow.rs, test_terminal_ops.rs
- test_selective_status.rs, test_amfv_part_i.rs, test_amfv_status.rs
- test_save_restore_flow.rs, test_read_char.rs

### Results ✅

**Build verification**: All remaining binaries compile successfully
**Functionality**: Core interpreter, compiler, and disassembler fully functional
**Maintenance**: Dramatically reduced build complexity and maintenance overhead
**Documentation**: Legacy tools remain available in git history if needed

The cleanup successfully removed obsolete debugging tools while preserving all essential functionality for ongoing development and testing.