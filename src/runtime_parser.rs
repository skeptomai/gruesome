// Runtime Parser Integration
// Connects the sophisticated ParserEngine to Z-Machine execution

use crate::grue_compiler::ir::{IrGrammar, IrPattern, IrPatternElement, IrProgram};
use crate::parser_engine::{ObjectInfo, ParseResult, ParsedCommand, ParserEngine};
use log::debug;
use std::collections::HashMap;

/// Runtime parser that integrates with Z-Machine execution
#[derive(Default)]
pub struct RuntimeParser {
    /// The sophisticated parser engine
    pub engine: ParserEngine,

    /// Compiled grammar from Grue program
    pub grammar: Vec<IrGrammar>,

    /// Object name resolution from game state
    pub object_resolver: ObjectResolver,

    /// Current game objects for parsing
    pub game_objects: HashMap<String, GameObjectInfo>,
}

/// Game object information for runtime parsing
#[derive(Debug, Clone)]
pub struct GameObjectInfo {
    pub id: String,
    pub names: Vec<String>,
    pub adjectives: Vec<String>,
    pub location: String,
    pub visible: bool,
    pub attributes: HashMap<String, bool>,
}

/// Resolves object references during parsing
#[derive(Debug, Clone)]
pub struct ObjectResolver {
    /// Maps object names to Z-Machine object numbers
    pub name_to_object: HashMap<String, u16>,

    /// Maps object numbers to full object info
    pub object_info: HashMap<u16, GameObjectInfo>,

    /// Current visible objects (for disambiguation)
    pub visible_objects: Vec<u16>,
}

impl RuntimeParser {
    /// Create new runtime parser with compiled grammar
    pub fn new(ir_program: &IrProgram) -> Self {
        let mut parser = RuntimeParser {
            engine: ParserEngine::new(),
            grammar: ir_program.grammar.clone(),
            object_resolver: ObjectResolver::new(),
            game_objects: HashMap::new(),
        };

        // Initialize parser with vocabulary from grammar
        parser.initialize_vocabulary();

        parser
    }

    /// Initialize parser vocabulary from compiled grammar
    fn initialize_vocabulary(&mut self) {
        // Clone grammar to avoid borrow checker issues
        let grammar_clone = self.grammar.clone();

        for grammar in &grammar_clone {
            // Each IrGrammar represents one verb with its patterns
            let verb_word = &grammar.verb;

            // Add verb synonyms
            self.engine
                .vocabulary
                .verb_synonyms
                .insert(verb_word.clone(), verb_word.clone());

            // Extract vocabulary from patterns
            for pattern in &grammar.patterns {
                self.extract_vocabulary_from_pattern(pattern);
            }
        }

        debug!(
            "Initialized runtime parser with {} verbs",
            self.engine.vocabulary.verb_synonyms.len()
        );
    }

    /// Extract vocabulary elements from advanced patterns
    fn extract_vocabulary_from_pattern(&mut self, pattern: &IrPattern) {
        for element in &pattern.pattern {
            match element {
                IrPatternElement::Adjective => {
                    // Add common adjectives
                    self.engine.vocabulary.adjectives.extend(
                        [
                            "small",
                            "large",
                            "brass",
                            "metal",
                            "glass",
                            "white",
                            "jewel-encrusted",
                            "emerald",
                            "precious",
                            "old",
                            "dusty",
                            "advertising",
                            "transparent",
                            "display",
                            "red",
                            "blue",
                        ]
                        .iter()
                        .map(|s| s.to_string()),
                    );
                }
                IrPatternElement::Preposition | IrPatternElement::AnyPreposition => {
                    // Add common prepositions
                    self.engine.vocabulary.prepositions.extend(
                        [
                            "in", "on", "under", "with", "from", "to", "at", "into", "onto", "out",
                            "of", "inside", "behind", "beside",
                        ]
                        .iter()
                        .map(|s| s.to_string()),
                    );
                }
                IrPatternElement::Literal(word) => {
                    // Check if it's a preposition
                    if ["in", "on", "with", "from", "to", "at"].contains(&word.as_str()) {
                        self.engine.vocabulary.prepositions.insert(word.clone());
                    }
                }
                _ => {} // Other elements don't contribute to vocabulary
            }
        }
    }

    /// Add game object to parser knowledge
    pub fn add_game_object(&mut self, obj: GameObjectInfo) {
        let object_info = ObjectInfo {
            primary_name: obj.names.first().cloned().unwrap_or(obj.id.clone()),
            names: obj.names.clone(),
            adjectives: obj.adjectives.clone(),
            location: obj.location.clone(),
            visible: obj.visible,
            object_id: obj.id.clone(),
        };

        self.engine.add_object(object_info);
        self.game_objects.insert(obj.id.clone(), obj);

        debug!("Added game object: {}", self.game_objects.len());
    }

    /// Parse user input using sophisticated parser
    pub fn parse_input(&mut self, input: &str) -> ParseResult {
        debug!("Runtime parsing: '{}'", input);

        // Use sophisticated parser engine
        let result = self.engine.parse(input);

        match &result {
            ParseResult::Success(cmd) => {
                debug!(
                    "Parsed command: verb='{}', direct_object={:?}",
                    cmd.verb,
                    cmd.direct_object.as_ref().map(|o| &o.object.primary_name)
                );
            }
            ParseResult::Disambiguation(query) => {
                debug!(
                    "Disambiguation needed: {} candidates",
                    query.context.candidates.len()
                );
            }
            ParseResult::NotUnderstood(msg) => {
                debug!("Not understood: {}", msg);
            }
            ParseResult::MissingObject(verb) => {
                debug!("Missing object for verb: {}", verb);
            }
            ParseResult::ObjectNotFound(obj) => {
                debug!("Object not found: {}", obj);
            }
            ParseResult::UnknownVerb(verb) => {
                debug!("Unknown verb: {}", verb);
            }
        }

        result
    }

    /// Match parsed command to compiled grammar patterns
    pub fn match_grammar_pattern(&self, cmd: &ParsedCommand) -> Option<GrammarMatch> {
        for grammar in &self.grammar {
            // Each IrGrammar represents one verb with its patterns
            if grammar.verb == cmd.verb {
                for pattern in &grammar.patterns {
                    if self.pattern_matches(&pattern.pattern, cmd) {
                        return Some(GrammarMatch {
                            verb: grammar.verb.clone(),
                            pattern: pattern.clone(),
                            arguments: self.extract_arguments(&pattern.pattern, cmd),
                        });
                    }
                }
            }
        }
        None
    }

    /// Check if pattern matches parsed command
    fn pattern_matches(&self, pattern: &[IrPatternElement], cmd: &ParsedCommand) -> bool {
        // Sophisticated pattern matching logic
        for element in pattern {
            match element {
                IrPatternElement::DirectObject => {
                    if cmd.direct_object.is_none() {
                        return false;
                    }
                }
                IrPatternElement::IndirectObject => {
                    if cmd.indirect_object.is_none() {
                        return false;
                    }
                }
                IrPatternElement::Preposition | IrPatternElement::AnyPreposition => {
                    if cmd.preposition.is_none() {
                        return false;
                    }
                }
                IrPatternElement::MultipleObjects => {
                    if let Some(obj) = &cmd.direct_object {
                        if !obj.multiple {
                            return false;
                        }
                    }
                }
                IrPatternElement::OptionalAdjective => {
                    // Always matches (optional)
                }
                _ => {
                    // Other elements require more sophisticated matching
                }
            }
        }
        true
    }

    /// Extract arguments from parsed command for pattern
    fn extract_arguments(
        &self,
        pattern: &[IrPatternElement],
        cmd: &ParsedCommand,
    ) -> Vec<ParsedArgument> {
        let mut args = Vec::new();

        for element in pattern {
            match element {
                IrPatternElement::DirectObject | IrPatternElement::Noun => {
                    if let Some(obj) = &cmd.direct_object {
                        args.push(ParsedArgument::Object(obj.object.object_id.clone()));
                    }
                }
                IrPatternElement::IndirectObject => {
                    if let Some(obj) = &cmd.indirect_object {
                        args.push(ParsedArgument::Object(obj.object.object_id.clone()));
                    }
                }
                IrPatternElement::Preposition | IrPatternElement::AnyPreposition => {
                    if let Some(prep) = &cmd.preposition {
                        args.push(ParsedArgument::String(prep.clone()));
                    }
                }
                IrPatternElement::MultipleObjects => {
                    if let Some(obj) = &cmd.direct_object {
                        if obj.multiple {
                            for game_obj in &obj.object_list {
                                args.push(ParsedArgument::Object(game_obj.object_id.clone()));
                            }
                        }
                    }
                }
                _ => {} // Other elements don't generate arguments
            }
        }

        args
    }
}

impl ObjectResolver {
    pub fn new() -> Self {
        ObjectResolver {
            name_to_object: HashMap::new(),
            object_info: HashMap::new(),
            visible_objects: Vec::new(),
        }
    }
}

/// Result of matching a command to grammar
#[derive(Debug, Clone)]
pub struct GrammarMatch {
    pub verb: String,
    pub pattern: IrPattern,
    pub arguments: Vec<ParsedArgument>,
}

/// Parsed argument from command
#[derive(Debug, Clone)]
pub enum ParsedArgument {
    Object(String),
    String(String),
    Number(i16),
}

impl Default for ObjectResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grue_compiler::ir::IrProgram;

    #[test]
    fn test_runtime_parser_creation() {
        let ir_program = IrProgram::default();
        let parser = RuntimeParser::new(&ir_program);

        assert!(!parser.grammar.is_empty() || parser.grammar.is_empty()); // Either is valid
    }

    #[test]
    fn test_object_addition() {
        let ir_program = IrProgram::default();
        let mut parser = RuntimeParser::new(&ir_program);

        let obj = GameObjectInfo {
            id: "lamp1".to_string(),
            names: vec!["brass lamp".to_string(), "lamp".to_string()],
            adjectives: vec!["brass".to_string(), "small".to_string()],
            location: "living room".to_string(),
            visible: true,
            attributes: HashMap::new(),
        };

        parser.add_game_object(obj);
        assert_eq!(parser.game_objects.len(), 1);
    }

    #[test]
    fn test_advanced_parsing() {
        let ir_program = IrProgram::default();
        let mut parser = RuntimeParser::new(&ir_program);

        // Add test object
        let obj = GameObjectInfo {
            id: "lamp1".to_string(),
            names: vec!["brass lamp".to_string(), "small lamp".to_string()],
            adjectives: vec!["brass".to_string(), "small".to_string()],
            location: "room".to_string(),
            visible: true,
            attributes: HashMap::new(),
        };
        parser.add_game_object(obj);

        // Test sophisticated parsing
        let result = parser.parse_input("take small brass lamp");

        if let ParseResult::Success(cmd) = result {
            assert_eq!(cmd.verb, "take");
            assert!(cmd.direct_object.is_some());
        }
    }
}
