//! Unit tests for generated opcode enums
//!
//! These tests verify that the type-safe opcode system works correctly.

#[cfg(test)]
mod tests {
    use super::super::codegen::InstructionForm;
    use super::super::opcodes::*;

    #[test]
    fn test_op0_raw_values() {
        assert_eq!(Op0::Quit.raw_value(), 0x0A);
        assert_eq!(Op0::NewLine.raw_value(), 0x0B);
        assert_eq!(Op0::Rtrue.raw_value(), 0x00);
        assert_eq!(Op0::Rfalse.raw_value(), 0x01);
    }

    #[test]
    fn test_op1_raw_values() {
        assert_eq!(Op1::PrintPaddr.raw_value(), 0x8D);
        assert_eq!(Op1::Jz.raw_value(), 0x00);
        assert_eq!(Op1::Load.raw_value(), 0x0E);
        assert_eq!(Op1::Jump.raw_value(), 0x0C);
    }

    #[test]
    fn test_op2_raw_values() {
        assert_eq!(Op2::Je.raw_value(), 0x01);
        assert_eq!(Op2::Add.raw_value(), 0x14);
        assert_eq!(Op2::Sub.raw_value(), 0x15);
        assert_eq!(Op2::GetNextProp.raw_value(), 0x13);
    }

    #[test]
    fn test_opvar_raw_values() {
        assert_eq!(OpVar::CallVs.raw_value(), 0x00);
        assert_eq!(OpVar::PutProp.raw_value(), 0x03);
        assert_eq!(OpVar::Sread.raw_value(), 0x04); // V1-V3 version (Aread is V4+)
        assert_eq!(OpVar::OutputStream.raw_value(), 0x13);
    }

    #[test]
    fn test_opcode_0x13_disambiguation() {
        // This is the critical bug from COMPILER_ARCHITECTURE.md
        // 0x13 can be either get_next_prop (2OP:19) or output_stream (VAR:243)

        let get_next_prop = Opcode::Op2(Op2::GetNextProp);
        let output_stream = Opcode::OpVar(OpVar::OutputStream);

        // Both have same raw value
        assert_eq!(get_next_prop.raw_value(), 0x13);
        assert_eq!(output_stream.raw_value(), 0x13);

        // But different store behavior
        assert!(
            get_next_prop.stores_result(),
            "get_next_prop MUST store result"
        );
        assert!(
            !output_stream.stores_result(),
            "output_stream MUST NOT store result"
        );

        // Different forms
        assert_eq!(get_next_prop.form(), InstructionForm::Long);
        assert_eq!(output_stream.form(), InstructionForm::Variable);
    }

    #[test]
    fn test_stores_result_metadata() {
        // Arithmetic operations store results
        assert!(Op2::Add.stores_result());
        assert!(Op2::Sub.stores_result());
        assert!(Op2::Mul.stores_result());
        assert!(Op2::Div.stores_result());

        // Branches don't store results
        assert!(!Op2::Je.stores_result());
        assert!(!Op2::Jl.stores_result());
        assert!(!Op2::Jg.stores_result());

        // Function calls store results
        assert!(OpVar::CallVs.stores_result());
        assert!(Op1::Load.stores_result());
    }

    #[test]
    fn test_branches_metadata() {
        // Branch instructions
        assert!(Op2::Je.branches());
        assert!(Op2::Jl.branches());
        assert!(Op2::Jg.branches());
        assert!(Op1::Jz.branches());
        assert!(Op2::TestAttr.branches());

        // Non-branch instructions
        assert!(!Op2::Add.branches());
        assert!(!OpVar::CallVs.branches());
        assert!(!Op0::Quit.branches());
    }

    #[test]
    fn test_version_requirements() {
        // V1 opcodes
        assert_eq!(Op0::Quit.min_version(), 1);
        assert_eq!(Op2::Add.min_version(), 1);
        assert_eq!(OpVar::CallVs.min_version(), 1);

        // V3+ opcodes
        assert_eq!(OpVar::SplitWindow.min_version(), 3);
        assert_eq!(OpVar::OutputStream.min_version(), 3);

        // V4+ opcodes
        assert_eq!(Op1::Call1s.min_version(), 4);
        assert_eq!(OpVar::ReadChar.min_version(), 4);

        // V5+ opcodes
        assert_eq!(Op2::Call2n.min_version(), 5);
        assert_eq!(OpVar::CallVn.min_version(), 5);
    }

    #[test]
    fn test_instruction_forms() {
        assert_eq!(Op0::Quit.form(), InstructionForm::Short);
        assert_eq!(Op1::PrintPaddr.form(), InstructionForm::Short);
        assert_eq!(Op2::Add.form(), InstructionForm::Long);
        assert_eq!(OpVar::CallVs.form(), InstructionForm::Variable);
    }

    #[test]
    fn test_combined_opcode_enum() {
        let quit = Opcode::Op0(Op0::Quit);
        let print_paddr = Opcode::Op1(Op1::PrintPaddr);
        let add = Opcode::Op2(Op2::Add);
        let call_vs = Opcode::OpVar(OpVar::CallVs);

        assert_eq!(quit.raw_value(), 0x0A);
        assert_eq!(print_paddr.raw_value(), 0x8D);
        assert_eq!(add.raw_value(), 0x14);
        assert_eq!(call_vs.raw_value(), 0x00);

        assert!(!quit.stores_result());
        assert!(!print_paddr.stores_result());
        assert!(add.stores_result());
        assert!(call_vs.stores_result());
    }

    #[test]
    fn test_convenience_constants() {
        // Test that convenience constants work
        assert_eq!(QUIT.raw_value(), 0x0A);
        assert_eq!(NEWLINE.raw_value(), 0x0B);
        assert_eq!(PRINTPADDR.raw_value(), 0x8D);
        assert_eq!(JE.raw_value(), 0x01);
        assert_eq!(ADD.raw_value(), 0x14);
        assert_eq!(CALLVS.raw_value(), 0x00);
    }

    #[test]
    fn test_opcode_metadata_trait() {
        // Test that the trait works for all enum types
        fn test_metadata<T: OpcodeMetadata>(opcode: T, expected_raw: u8) {
            assert_eq!(opcode.raw_value(), expected_raw);
        }

        test_metadata(Op0::Quit, 0x0A);
        test_metadata(Op1::PrintPaddr, 0x8D);
        test_metadata(Op2::Add, 0x14);
        test_metadata(OpVar::CallVs, 0x00);
    }

    #[test]
    fn test_const_fn_evaluation() {
        // These should be evaluable at compile time
        const QUIT_VALUE: u8 = Op0::Quit.raw_value();
        const ADD_STORES: bool = Op2::Add.stores_result();
        const JE_BRANCHES: bool = Op2::Je.branches();
        const QUIT_VERSION: u8 = Op0::Quit.min_version();

        assert_eq!(QUIT_VALUE, 0x0A);
        assert!(ADD_STORES);
        assert!(JE_BRANCHES);
        assert_eq!(QUIT_VERSION, 1);
    }

    #[test]
    fn test_hash_and_eq() {
        use std::collections::HashSet;

        // Test that opcodes can be used in HashSets
        let mut opcodes = HashSet::new();
        opcodes.insert(Op0::Quit);
        opcodes.insert(Op0::NewLine);
        opcodes.insert(Op0::Quit); // Duplicate

        assert_eq!(opcodes.len(), 2);
        assert!(opcodes.contains(&Op0::Quit));
        assert!(opcodes.contains(&Op0::NewLine));
    }

    #[test]
    fn test_all_op0_opcodes_unique() {
        let opcodes = vec![
            Op0::Rtrue,
            Op0::Rfalse,
            Op0::Print,
            Op0::PrintRet,
            Op0::Nop,
            Op0::Save,
            Op0::Restore,
            Op0::Restart,
            Op0::RetPopped,
            Op0::Pop,
            Op0::Quit,
            Op0::NewLine,
            Op0::ShowStatus,
            Op0::Verify,
        ];

        let mut values = std::collections::HashSet::new();
        for opcode in opcodes {
            assert!(
                values.insert(opcode.raw_value()),
                "Duplicate opcode value: 0x{:02X}",
                opcode.raw_value()
            );
        }
    }

    #[test]
    fn test_all_op2_opcodes_unique() {
        let opcodes = vec![
            Op2::Je,
            Op2::Jl,
            Op2::Jg,
            Op2::DecChk,
            Op2::IncChk,
            Op2::Jin,
            Op2::Test,
            Op2::Or,
            Op2::And,
            Op2::TestAttr,
            Op2::SetAttr,
            Op2::ClearAttr,
            Op2::Store,
            Op2::InsertObj,
            Op2::Loadw,
            Op2::Loadb,
            Op2::GetProp,
            Op2::GetPropAddr,
            Op2::GetNextProp,
            Op2::Add,
            Op2::Sub,
            Op2::Mul,
            Op2::Div,
            Op2::Mod,
        ];

        let mut values = std::collections::HashSet::new();
        for opcode in opcodes {
            assert!(
                values.insert(opcode.raw_value()),
                "Duplicate opcode value: 0x{:02X}",
                opcode.raw_value()
            );
        }
    }
}
