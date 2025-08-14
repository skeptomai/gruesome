// Advanced Parser Engine for Zork I-Level Text Adventure Games
// Supports multi-word nouns, disambiguation, adjectives, prepositions, and multiple objects

use log::debug;
use std::collections::{HashMap, HashSet};

/// Advanced parser engine for text adventure games
#[derive(Debug, Clone)]
pub struct ParserEngine {
    /// Dictionary of known words and their types
    pub vocabulary: Vocabulary,

    /// Object name mappings (from game world)
    pub object_names: HashMap<String, ObjectInfo>,

    /// Current disambiguation context
    pub disambiguation_state: Option<DisambiguationState>,

    /// Parser configuration
    pub config: ParserConfig,
}

/// Comprehensive vocabulary system
#[derive(Debug, Clone)]
pub struct Vocabulary {
    /// Adjectives that can modify nouns
    pub adjectives: HashSet<String>,

    /// Prepositions for spatial/temporal relationships
    pub prepositions: HashSet<String>,

    /// Articles (ignored in parsing but recognized)
    pub articles: HashSet<String>,

    /// Pronouns and their resolution context
    pub pronouns: HashMap<String, PronounType>,

    /// Conjunctions for multiple objects
    pub conjunctions: HashSet<String>,

    /// Verb synonyms mapping
    pub verb_synonyms: HashMap<String, String>,

    /// Noun synonyms mapping  
    pub noun_synonyms: HashMap<String, String>,
}

/// Information about a game object for parsing
#[derive(Debug, Clone)]
pub struct ObjectInfo {
    /// Primary name of the object
    pub primary_name: String,

    /// All names this object responds to
    pub names: Vec<String>,

    /// Adjectives that apply to this object
    pub adjectives: Vec<String>,

    /// Current location of object (for disambiguation)
    pub location: String,

    /// Whether object is currently visible to player
    pub visible: bool,

    /// Object ID for game reference
    pub object_id: String,
}

/// Types of pronouns and their behavior
#[derive(Debug, Clone, PartialEq)]
pub enum PronounType {
    It,   // Singular neuter
    Them, // Plural
    Him,  // Singular masculine
    Her,  // Singular feminine
    All,  // Everything visible
}

/// Disambiguation state when multiple objects match
#[derive(Debug, Clone)]
pub struct DisambiguationState {
    /// Original user input that caused ambiguity
    pub original_input: String,

    /// List of possible object matches
    pub candidates: Vec<ObjectInfo>,

    /// The verb being attempted
    pub verb: String,

    /// Additional context (preposition, etc.)
    pub context: Option<String>,
}

/// Parser configuration options
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Maximum words to consider for multi-word nouns
    pub max_noun_words: usize,

    /// Whether to use fuzzy matching for typos
    pub fuzzy_matching: bool,

    /// Whether to automatically resolve single disambiguation
    pub auto_resolve_single: bool,

    /// Whether to use pronoun context
    pub use_pronouns: bool,

    /// Maximum disambiguation candidates to show
    pub max_disambiguation_candidates: usize,
}

/// Result of parsing user input
#[derive(Debug, Clone)]
pub enum ParseResult {
    /// Successfully parsed command
    Success(Box<ParsedCommand>),

    /// Multiple objects match - needs disambiguation  
    Disambiguation(DisambiguationQuery),

    /// Input not understood
    NotUnderstood(String),

    /// Missing object (e.g., "take" without object)
    MissingObject(String),

    /// Object not found
    ObjectNotFound(String),

    /// Verb not recognized
    UnknownVerb(String),
}

/// Successfully parsed command ready for execution
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    /// The verb to execute
    pub verb: String,

    /// Direct object (if any)
    pub direct_object: Option<ParsedObject>,

    /// Indirect object (if any, e.g., "put lamp in mailbox")
    pub indirect_object: Option<ParsedObject>,

    /// Preposition connecting objects
    pub preposition: Option<String>,

    /// Additional context or modifiers
    pub modifiers: Vec<String>,
}

/// A fully parsed object with context
#[derive(Debug, Clone)]
pub struct ParsedObject {
    /// The object information
    pub object: ObjectInfo,

    /// Adjectives used to specify this object  
    pub specified_adjectives: Vec<String>,

    /// Whether this represents multiple objects ("all", "everything")
    pub multiple: bool,

    /// If multiple, the list of objects
    pub object_list: Vec<ObjectInfo>,
}

/// Disambiguation query to present to user
#[derive(Debug, Clone)]
pub struct DisambiguationQuery {
    /// Question to ask user
    pub question: String,

    /// List of options with their identifiers
    pub options: Vec<(String, ObjectInfo)>,

    /// Context to resume parsing after disambiguation
    pub context: DisambiguationState,
}

impl ParserEngine {
    /// Create a new parser engine with default configuration
    pub fn new() -> Self {
        Self {
            vocabulary: Vocabulary::default(),
            object_names: HashMap::new(),
            disambiguation_state: None,
            config: ParserConfig::default(),
        }
    }

    /// Add an object to the parser's knowledge
    pub fn add_object(&mut self, object: ObjectInfo) {
        // Add all names for this object
        for name in &object.names {
            self.object_names.insert(name.clone(), object.clone());
        }

        // Also add adjective + noun combinations
        for adjective in &object.adjectives {
            for name in &object.names {
                let combined = format!("{} {}", adjective, name);
                self.object_names.insert(combined, object.clone());
            }
        }

        debug!(
            "Added object '{}' with {} names",
            object.primary_name,
            object.names.len()
        );
    }

    /// Parse user input into a command
    pub fn parse(&mut self, input: &str) -> ParseResult {
        debug!("Parsing input: '{}'", input);

        // Normalize input (lowercase, trim, handle contractions)
        let normalized = self.normalize_input(input);
        let words = self.tokenize(&normalized);

        debug!("Tokenized: {:?}", words);

        // Handle empty input
        if words.is_empty() {
            return ParseResult::NotUnderstood("I didn't understand that.".to_string());
        }

        // Extract verb (first word typically)
        let verb = &words[0];
        let verb_canonical = self.resolve_verb_synonym(verb);

        // Extract objects and prepositions from remaining words
        let remaining_words = &words[1..];

        match self.parse_command_structure(&verb_canonical, remaining_words) {
            Ok(command) => ParseResult::Success(Box::new(command)),
            Err(parse_error) => self.handle_parse_error(parse_error, input),
        }
    }

    /// Normalize user input for consistent processing
    fn normalize_input(&self, input: &str) -> String {
        input
            .to_lowercase()
            .trim()
            .replace("'", "") // Remove apostrophes
            .replace(",", "") // Remove commas
            .replace(".", "") // Remove periods
    }

    /// Split input into words
    fn tokenize(&self, input: &str) -> Vec<String> {
        input.split_whitespace().map(|s| s.to_string()).collect()
    }

    /// Resolve verb synonyms to canonical form
    fn resolve_verb_synonym(&self, verb: &str) -> String {
        self.vocabulary
            .verb_synonyms
            .get(verb)
            .cloned()
            .unwrap_or_else(|| verb.to_string())
    }

    /// Parse the command structure after verb extraction
    fn parse_command_structure(
        &mut self,
        verb: &str,
        words: &[String],
    ) -> Result<ParsedCommand, ParseError> {
        // Handle commands with no objects (like "inventory", "look")
        if words.is_empty() {
            return Ok(ParsedCommand {
                verb: verb.to_string(),
                direct_object: None,
                indirect_object: None,
                preposition: None,
                modifiers: Vec::new(),
            });
        }

        // Look for prepositions to split direct/indirect objects
        let preposition_index = self.find_preposition(words);

        let (direct_words, indirect_words, preposition) = if let Some(prep_idx) = preposition_index
        {
            let direct = &words[0..prep_idx];
            let prep = &words[prep_idx];
            let indirect = &words[prep_idx + 1..];
            (direct, indirect, Some(prep.clone()))
        } else {
            (words, &[] as &[String], None)
        };

        // Parse direct object
        let direct_object = if !direct_words.is_empty() {
            Some(self.parse_object(direct_words)?)
        } else {
            None
        };

        // Parse indirect object
        let indirect_object = if !indirect_words.is_empty() {
            Some(self.parse_object(indirect_words)?)
        } else {
            None
        };

        Ok(ParsedCommand {
            verb: verb.to_string(),
            direct_object,
            indirect_object,
            preposition,
            modifiers: Vec::new(),
        })
    }

    /// Find preposition in word list
    fn find_preposition(&self, words: &[String]) -> Option<usize> {
        for (i, word) in words.iter().enumerate() {
            if self.vocabulary.prepositions.contains(word) {
                return Some(i);
            }
        }
        None
    }

    /// Parse a sequence of words into an object
    fn parse_object(&mut self, words: &[String]) -> Result<ParsedObject, ParseError> {
        debug!("Parsing object from words: {:?}", words);

        // Handle special cases
        if words.len() == 1 && (words[0] == "all" || words[0] == "everything") {
            return self.parse_multiple_objects();
        }

        // Try different word combinations (longest first for multi-word nouns)
        for len in (1..=words.len().min(self.config.max_noun_words)).rev() {
            for start in 0..=(words.len() - len) {
                let phrase = words[start..start + len].join(" ");

                if let Some(object) = self.object_names.get(&phrase).cloned() {
                    // Check if object is accessible
                    if !object.visible {
                        return Err(ParseError::ObjectNotVisible(phrase));
                    }

                    // Extract any adjectives that were used
                    let specified_adjectives = self.extract_adjectives(words);

                    return Ok(ParsedObject {
                        object,
                        specified_adjectives,
                        multiple: false,
                        object_list: Vec::new(),
                    });
                }
            }
        }

        // No exact match found - try fuzzy matching or report error
        let phrase = words.join(" ");
        Err(ParseError::ObjectNotFound(phrase))
    }

    /// Parse "all" or "everything" into multiple objects
    fn parse_multiple_objects(&self) -> Result<ParsedObject, ParseError> {
        let visible_objects: Vec<ObjectInfo> = self
            .object_names
            .values()
            .filter(|obj| obj.visible)
            .cloned()
            .collect();

        if visible_objects.is_empty() {
            return Err(ParseError::NoObjectsAvailable);
        }

        // Create a dummy object representing "all"
        let all_object = ObjectInfo {
            primary_name: "all".to_string(),
            names: vec!["all".to_string(), "everything".to_string()],
            adjectives: Vec::new(),
            location: "multiple".to_string(),
            visible: true,
            object_id: "all".to_string(),
        };

        Ok(ParsedObject {
            object: all_object,
            specified_adjectives: Vec::new(),
            multiple: true,
            object_list: visible_objects,
        })
    }

    /// Extract adjectives from word list
    fn extract_adjectives(&self, words: &[String]) -> Vec<String> {
        words
            .iter()
            .filter(|word| self.vocabulary.adjectives.contains(*word))
            .cloned()
            .collect()
    }

    /// Handle parsing errors and convert to appropriate results
    fn handle_parse_error(&self, error: ParseError, original_input: &str) -> ParseResult {
        match error {
            ParseError::ObjectNotFound(obj) => ParseResult::ObjectNotFound(obj),
            ParseError::ObjectNotVisible(obj) => {
                ParseResult::ObjectNotFound(format!("You can't see any {} here.", obj))
            }
            ParseError::NoObjectsAvailable => {
                ParseResult::NotUnderstood("There's nothing here.".to_string())
            }
            ParseError::AmbiguousObject(candidates) => {
                // Create disambiguation query
                let question = format!(
                    "Which {} do you mean?",
                    candidates
                        .first()
                        .map(|c| &c.primary_name)
                        .unwrap_or(&"object".to_string())
                );

                let options = candidates
                    .into_iter()
                    .enumerate()
                    .map(|(i, obj)| (format!("{}. {}", i + 1, obj.primary_name), obj))
                    .collect();

                ParseResult::Disambiguation(DisambiguationQuery {
                    question,
                    options,
                    context: DisambiguationState {
                        original_input: original_input.to_string(),
                        candidates: Vec::new(), // Will be filled by caller
                        verb: "unknown".to_string(), // Will be filled by caller
                        context: None,
                    },
                })
            }
        }
    }
}

/// Errors that can occur during parsing
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Object name not found in vocabulary
    ObjectNotFound(String),

    /// Object exists but not currently visible
    ObjectNotVisible(String),

    /// Multiple objects match the description
    AmbiguousObject(Vec<ObjectInfo>),

    /// No objects available for "all"
    NoObjectsAvailable,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            max_noun_words: 3,
            fuzzy_matching: false,
            auto_resolve_single: true,
            use_pronouns: true,
            max_disambiguation_candidates: 5,
        }
    }
}

impl Default for Vocabulary {
    fn default() -> Self {
        let mut vocab = Self {
            adjectives: HashSet::new(),
            prepositions: HashSet::new(),
            articles: HashSet::new(),
            pronouns: HashMap::new(),
            conjunctions: HashSet::new(),
            verb_synonyms: HashMap::new(),
            noun_synonyms: HashMap::new(),
        };

        // Add standard English words
        vocab.add_standard_vocabulary();
        vocab
    }
}

impl Vocabulary {
    /// Add standard English vocabulary for text adventures
    fn add_standard_vocabulary(&mut self) {
        // Common adjectives
        let adjectives = vec![
            "small", "large", "big", "little", "tiny", "huge", "red", "blue", "green", "yellow",
            "white", "black", "brown", "silver", "gold", "wooden", "metal", "glass", "plastic",
            "open", "closed", "locked", "unlocked", "empty", "full", "clean", "dirty", "old",
            "new", "hot", "cold", "warm", "cool", "bright", "dark", "heavy", "light", "sharp",
            "dull",
        ];

        for adj in adjectives {
            self.adjectives.insert(adj.to_string());
        }

        // Common prepositions
        let prepositions = vec![
            "in", "on", "under", "over", "behind", "beside", "with", "to", "from", "into", "onto",
            "through", "around", "across", "between", "among", "against", "near", "by",
        ];

        for prep in prepositions {
            self.prepositions.insert(prep.to_string());
        }

        // Articles
        let articles = vec!["the", "a", "an", "some", "any"];
        for article in articles {
            self.articles.insert(article.to_string());
        }

        // Pronouns
        self.pronouns.insert("it".to_string(), PronounType::It);
        self.pronouns.insert("them".to_string(), PronounType::Them);
        self.pronouns.insert("him".to_string(), PronounType::Him);
        self.pronouns.insert("her".to_string(), PronounType::Her);
        self.pronouns.insert("all".to_string(), PronounType::All);

        // Conjunctions
        let conjunctions = vec!["and", "then", ","];
        for conj in conjunctions {
            self.conjunctions.insert(conj.to_string());
        }

        // Common verb synonyms
        self.verb_synonyms
            .insert("get".to_string(), "take".to_string());
        self.verb_synonyms
            .insert("pick".to_string(), "take".to_string());
        self.verb_synonyms
            .insert("grab".to_string(), "take".to_string());
        self.verb_synonyms
            .insert("l".to_string(), "look".to_string());
        self.verb_synonyms
            .insert("x".to_string(), "examine".to_string());
        self.verb_synonyms
            .insert("i".to_string(), "inventory".to_string());
        self.verb_synonyms
            .insert("inv".to_string(), "inventory".to_string());
    }
}

impl Default for ParserEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = ParserEngine::new();
        assert!(parser.vocabulary.adjectives.contains("red"));
        assert!(parser.vocabulary.prepositions.contains("in"));
    }

    #[test]
    fn test_object_addition() {
        let mut parser = ParserEngine::new();

        let lamp = ObjectInfo {
            primary_name: "brass lamp".to_string(),
            names: vec!["lamp".to_string(), "brass lamp".to_string()],
            adjectives: vec!["brass".to_string(), "small".to_string()],
            location: "living room".to_string(),
            visible: true,
            object_id: "lamp1".to_string(),
        };

        parser.add_object(lamp);

        assert!(parser.object_names.contains_key("lamp"));
        assert!(parser.object_names.contains_key("brass lamp"));
        assert!(parser.object_names.contains_key("brass lamp")); // adjective + noun
    }

    #[test]
    fn test_simple_parsing() {
        let mut parser = ParserEngine::new();

        // Add a test object
        let lamp = ObjectInfo {
            primary_name: "lamp".to_string(),
            names: vec!["lamp".to_string()],
            adjectives: vec!["brass".to_string()],
            location: "room".to_string(),
            visible: true,
            object_id: "lamp1".to_string(),
        };

        parser.add_object(lamp);

        // Test parsing
        let result = parser.parse("take lamp");

        match result {
            ParseResult::Success(cmd) => {
                assert_eq!(cmd.verb, "take");
                assert!(cmd.direct_object.is_some());
                assert_eq!(cmd.direct_object.unwrap().object.primary_name, "lamp");
            }
            _ => panic!("Expected successful parse, got {:?}", result),
        }
    }
}
