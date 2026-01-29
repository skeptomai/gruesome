// IR Generator - Grammar Pattern Generation
//
// Extracted from ir_generator.rs as part of modularization effort.
// Handles conversion of AST grammar declarations to IR grammar structures.

use crate::grue_compiler::error::CompilerError;

use super::{IrGenerator, IrGrammar, IrHandler, IrPattern, IrPatternElement};

impl IrGenerator {
    /// Generate IR grammar patterns from AST grammar declarations
    ///
    /// Converts verb patterns and handlers from AST into IR representation.
    /// Handles polymorphic dispatch function resolution for handler calls.
    pub(super) fn generate_grammar(
        &mut self,
        grammar: crate::grue_compiler::ast::GrammarDecl,
    ) -> Result<Vec<IrGrammar>, CompilerError> {
        let mut ir_grammar = Vec::new();

        for verb in grammar.verbs {
            let mut patterns = Vec::new();

            for pattern in verb.patterns {
                let ir_pattern_elements: Vec<IrPatternElement> = pattern
                    .pattern
                    .into_iter()
                    .map(|elem| match elem {
                        crate::grue_compiler::ast::PatternElement::Literal(s) => {
                            IrPatternElement::Literal(s)
                        }
                        crate::grue_compiler::ast::PatternElement::Noun => IrPatternElement::Noun,
                        crate::grue_compiler::ast::PatternElement::Default => {
                            IrPatternElement::Default
                        }
                        // Enhanced parser elements - full support for Zork I-level parsing
                        crate::grue_compiler::ast::PatternElement::Adjective => {
                            IrPatternElement::Adjective
                        }
                        crate::grue_compiler::ast::PatternElement::MultiWordNoun => {
                            IrPatternElement::MultiWordNoun
                        }
                        crate::grue_compiler::ast::PatternElement::Preposition => {
                            IrPatternElement::Preposition
                        }
                        crate::grue_compiler::ast::PatternElement::MultipleObjects => {
                            IrPatternElement::MultipleObjects
                        }
                        crate::grue_compiler::ast::PatternElement::DirectObject => {
                            IrPatternElement::DirectObject
                        }
                        crate::grue_compiler::ast::PatternElement::IndirectObject => {
                            IrPatternElement::IndirectObject
                        }
                        crate::grue_compiler::ast::PatternElement::OptionalAdjective => {
                            IrPatternElement::OptionalAdjective
                        }
                        crate::grue_compiler::ast::PatternElement::AnyPreposition => {
                            IrPatternElement::AnyPreposition
                        }
                        crate::grue_compiler::ast::PatternElement::NumberedNoun => {
                            IrPatternElement::NumberedNoun
                        }
                    })
                    .collect();

                let ir_handler = match pattern.handler {
                    crate::grue_compiler::ast::Handler::FunctionCall(name, args) => {
                        // Convert arguments to IR values
                        let mut ir_args = Vec::new();
                        for arg in args {
                            let ir_value = self.expr_to_ir_value(arg)?;
                            ir_args.push(ir_value);
                        }

                        // CRITICAL FIX: Look up function ID using symbol table resolution
                        // Previously used placeholder function ID 0, causing "Routine ID 0 not found" errors
                        // during code generation. Now properly resolves function names to their assigned IR IDs.
                        // This enables grammar pattern handlers like handle_look() to be correctly called.

                        // POLYMORPHIC DISPATCH FIX: Use dispatch function if available
                        let func_id = if let Some(&dispatch_id) = self.dispatch_functions.get(&name)
                        {
                            log::debug!(
                                "🎯 Grammar using dispatch function for '{}': ID {}",
                                name,
                                dispatch_id
                            );
                            dispatch_id
                        } else if let Some(&id) = self.symbol_ids.get(&name) {
                            log::debug!(
                                "🎯 Grammar using original function for '{}': ID {}",
                                name,
                                id
                            );
                            id
                        } else {
                            return Err(CompilerError::SemanticError(
                                format!(
                                    "Grammar handler function '{}' not found. All functions must be defined before grammar patterns.",
                                    name
                                ),
                                0,
                            ));
                        };

                        IrHandler::FunctionCall(func_id, ir_args)
                    }
                    crate::grue_compiler::ast::Handler::Block(block) => {
                        let ir_block = self.generate_block(block)?;
                        IrHandler::Block(ir_block)
                    }
                };

                patterns.push(IrPattern {
                    pattern: ir_pattern_elements,
                    handler: ir_handler,
                });
            }

            ir_grammar.push(IrGrammar {
                verb: verb.word,
                patterns,
            });
        }

        Ok(ir_grammar)
    }
}
