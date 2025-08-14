// Grue Language Recursive Descent Parser

use crate::grue_compiler::ast::*;
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::lexer::{Token, TokenKind};
use std::collections::HashMap;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, CompilerError> {
        let mut items = Vec::new();

        while !self.is_at_end() {
            // Skip newlines at top level
            if self.check(&TokenKind::Newline) {
                self.advance();
                continue;
            }

            items.push(self.parse_item()?);
        }

        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Result<Item, CompilerError> {
        match &self.peek().kind {
            TokenKind::World => Ok(Item::World(self.parse_world_decl()?)),
            TokenKind::Grammar => Ok(Item::Grammar(self.parse_grammar_decl()?)),
            TokenKind::Function => Ok(Item::Function(self.parse_function_decl()?)),
            TokenKind::Init => Ok(Item::Init(self.parse_init_decl()?)),
            _ => {
                let token = self.peek();
                Err(CompilerError::ExpectedToken(
                    "world, grammar, fn, or init".to_string(),
                    format!("{:?}", token.kind),
                    token.position,
                ))
            }
        }
    }

    fn parse_world_decl(&mut self) -> Result<WorldDecl, CompilerError> {
        self.consume(TokenKind::World, "Expected 'world'")?;
        self.consume(TokenKind::LeftBrace, "Expected '{' after 'world'")?;

        let mut rooms = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Skip newlines
            if self.check(&TokenKind::Newline) {
                self.advance();
                continue;
            }

            rooms.push(self.parse_room_decl()?);
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after world body")?;

        Ok(WorldDecl { rooms })
    }

    fn parse_room_decl(&mut self) -> Result<RoomDecl, CompilerError> {
        self.consume(TokenKind::Room, "Expected 'room'")?;

        let identifier = self.consume_identifier("Expected room identifier")?;
        let display_name = self.consume_string("Expected room display name")?;

        self.consume(TokenKind::LeftBrace, "Expected '{' after room declaration")?;

        let mut description = String::new();
        let mut objects = Vec::new();
        let mut exits = HashMap::new();
        let mut on_enter = None;
        let mut on_exit = None;
        let mut on_look = None;

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Skip newlines
            if self.check(&TokenKind::Newline) {
                self.advance();
                continue;
            }

            match &self.peek().kind {
                TokenKind::Desc => {
                    self.advance();
                    self.consume(TokenKind::Colon, "Expected ':' after 'desc'")?;
                    description = self.consume_string("Expected room description")?;
                }
                TokenKind::Object => {
                    objects.push(self.parse_object_decl()?);
                }
                TokenKind::Exits => {
                    exits = self.parse_exits()?;
                }
                TokenKind::OnEnter => {
                    self.advance();
                    self.consume(TokenKind::Colon, "Expected ':' after 'on_enter'")?;
                    on_enter = Some(self.parse_block()?);
                }
                TokenKind::OnExit => {
                    self.advance();
                    self.consume(TokenKind::Colon, "Expected ':' after 'on_exit'")?;
                    on_exit = Some(self.parse_block()?);
                }
                TokenKind::OnLook => {
                    self.advance();
                    self.consume(TokenKind::Colon, "Expected ':' after 'on_look'")?;
                    on_look = Some(self.parse_block()?);
                }
                _ => {
                    let token = self.peek();
                    return Err(CompilerError::ExpectedToken(
                        "desc, object, exits, on_enter, on_exit, or on_look".to_string(),
                        format!("{:?}", token.kind),
                        token.position,
                    ));
                }
            }
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after room body")?;

        Ok(RoomDecl {
            identifier,
            display_name,
            description,
            objects,
            exits,
            on_enter,
            on_exit,
            on_look,
        })
    }

    fn parse_object_decl(&mut self) -> Result<ObjectDecl, CompilerError> {
        self.consume(TokenKind::Object, "Expected 'object'")?;

        let identifier = self.consume_identifier("Expected object identifier")?;

        self.consume(TokenKind::LeftBrace, "Expected '{' after object identifier")?;

        let mut names = Vec::new();
        let mut description = String::new();
        let mut properties = HashMap::new();
        let mut contains = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Skip newlines
            if self.check(&TokenKind::Newline) {
                self.advance();
                continue;
            }

            match &self.peek().kind {
                TokenKind::Names => {
                    self.advance();
                    self.consume(TokenKind::Colon, "Expected ':' after 'names'")?;
                    names = self.parse_string_array()?;
                }
                TokenKind::Desc => {
                    self.advance();
                    self.consume(TokenKind::Colon, "Expected ':' after 'desc'")?;
                    description = self.parse_expression_as_string()?;
                }
                TokenKind::Contains => {
                    self.advance();
                    self.consume(TokenKind::LeftBrace, "Expected '{' after 'contains'")?;

                    while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
                        if self.check(&TokenKind::Newline) {
                            self.advance();
                            continue;
                        }
                        contains.push(self.parse_object_decl()?);
                    }

                    self.consume(TokenKind::RightBrace, "Expected '}' after contains body")?;
                }
                TokenKind::Identifier(key) => {
                    let property_key = key.clone();
                    self.advance();
                    self.consume(TokenKind::Colon, "Expected ':' after property name")?;
                    let value = self.parse_property_value()?;
                    properties.insert(property_key, value);
                }
                _ => {
                    let token = self.peek();
                    return Err(CompilerError::ExpectedToken(
                        "names, desc, contains, or property name".to_string(),
                        format!("{:?}", token.kind),
                        token.position,
                    ));
                }
            }
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after object body")?;

        Ok(ObjectDecl {
            identifier,
            names,
            description,
            properties,
            attributes: Vec::new(), // TODO: Parse from object syntax
            numbered_properties: HashMap::new(), // TODO: Parse from object syntax
            contains,
        })
    }

    fn parse_exits(&mut self) -> Result<HashMap<String, ExitTarget>, CompilerError> {
        self.consume(TokenKind::Exits, "Expected 'exits'")?;
        self.consume(TokenKind::Colon, "Expected ':' after 'exits'")?;
        self.consume(TokenKind::LeftBrace, "Expected '{' after 'exits:'")?;

        let mut exits = HashMap::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Skip newlines and commas
            if self.check(&TokenKind::Newline) || self.check(&TokenKind::Comma) {
                self.advance();
                continue;
            }

            let direction = self.consume_identifier("Expected direction identifier")?;
            self.consume(TokenKind::Colon, "Expected ':' after direction")?;

            // Parse the target (can be identifier or function call)
            let expr = self.parse_expression()?;
            let target = match expr {
                Expr::Identifier(room_name) => ExitTarget::Room(room_name),
                Expr::FunctionCall { name, arguments } if name == "blocked" => {
                    if let Some(Expr::String(message)) = arguments.first() {
                        ExitTarget::Blocked(message.clone())
                    } else {
                        return Err(CompilerError::ParseError(
                            "blocked() requires a string message".to_string(),
                            self.previous().position,
                        ));
                    }
                }
                _ => {
                    return Err(CompilerError::ParseError(
                        "Exit target must be room identifier or blocked() call".to_string(),
                        self.previous().position,
                    ));
                }
            };

            exits.insert(direction, target);
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after exits body")?;

        Ok(exits)
    }

    fn parse_string_array(&mut self) -> Result<Vec<String>, CompilerError> {
        self.consume(TokenKind::LeftBracket, "Expected '[' for string array")?;

        let mut strings = Vec::new();

        while !self.check(&TokenKind::RightBracket) && !self.is_at_end() {
            if self.check(&TokenKind::Comma) {
                self.advance();
                continue;
            }

            strings.push(self.consume_string("Expected string in array")?);
        }

        self.consume(TokenKind::RightBracket, "Expected ']' after string array")?;

        Ok(strings)
    }

    fn parse_property_value(&mut self) -> Result<PropertyValue, CompilerError> {
        match &self.peek().kind {
            TokenKind::True => {
                self.advance();
                Ok(PropertyValue::Boolean(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(PropertyValue::Boolean(false))
            }
            TokenKind::IntegerLiteral(val) => {
                let value = *val;
                self.advance();
                Ok(PropertyValue::Integer(value))
            }
            TokenKind::StringLiteral(val) => {
                let value = val.clone();
                self.advance();
                Ok(PropertyValue::String(value))
            }
            _ => {
                // For now, treat everything else as an error
                let token = self.peek();
                Err(CompilerError::ExpectedToken(
                    "boolean, integer, or string value".to_string(),
                    format!("{:?}", token.kind),
                    token.position,
                ))
            }
        }
    }

    fn parse_expression_as_string(&mut self) -> Result<String, CompilerError> {
        // Parse expression and convert to string representation
        // This is a simplified approach - in a real compiler, we'd store the expression
        let _expr = self.parse_expression()?;
        // For now, return a placeholder string
        // TODO: Properly evaluate string expressions during semantic analysis
        Ok("[expression]".to_string())
    }

    fn parse_grammar_decl(&mut self) -> Result<GrammarDecl, CompilerError> {
        self.consume(TokenKind::Grammar, "Expected 'grammar'")?;
        self.consume(TokenKind::LeftBrace, "Expected '{' after 'grammar'")?;

        let mut verbs = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Skip newlines
            if self.check(&TokenKind::Newline) {
                self.advance();
                continue;
            }

            verbs.push(self.parse_verb_decl()?);
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after grammar body")?;

        Ok(GrammarDecl {
            verbs,
            vocabulary: None, // TODO: Parse vocabulary declarations in future
        })
    }

    fn parse_verb_decl(&mut self) -> Result<VerbDecl, CompilerError> {
        self.consume(TokenKind::Verb, "Expected 'verb'")?;
        let word = self.consume_string("Expected verb word")?;

        self.consume(TokenKind::LeftBrace, "Expected '{' after verb word")?;

        let mut patterns = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Skip newlines and commas
            if self.check(&TokenKind::Newline) || self.check(&TokenKind::Comma) {
                self.advance();
                continue;
            }

            patterns.push(self.parse_verb_pattern()?);
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after verb body")?;

        Ok(VerbDecl { word, patterns })
    }

    fn parse_verb_pattern(&mut self) -> Result<VerbPattern, CompilerError> {
        // For now, implement simplified pattern parsing
        let mut pattern = Vec::new();

        // Parse pattern elements until we hit '=>'
        while !self.check(&TokenKind::Arrow) && !self.is_at_end() {
            match &self.peek().kind {
                TokenKind::StringLiteral(val) => {
                    pattern.push(PatternElement::Literal(val.clone()));
                    self.advance();
                }
                TokenKind::Identifier(name) if name == "noun" => {
                    pattern.push(PatternElement::Noun);
                    self.advance();
                }
                TokenKind::Identifier(name) if name == "default" => {
                    pattern.push(PatternElement::Default);
                    self.advance();
                }
                TokenKind::Plus => {
                    self.advance(); // Skip '+' connector
                }
                _ => break,
            }
        }

        self.consume(TokenKind::Arrow, "Expected '=>' after pattern")?;

        let handler = self.parse_handler()?;

        Ok(VerbPattern { pattern, handler })
    }

    fn parse_handler(&mut self) -> Result<Handler, CompilerError> {
        if self.check(&TokenKind::LeftBrace) {
            Ok(Handler::Block(self.parse_block()?))
        } else {
            // Parse function call
            let expr = self.parse_expression()?;
            match expr {
                Expr::FunctionCall { name, arguments } => {
                    Ok(Handler::FunctionCall(name, arguments))
                }
                _ => Err(CompilerError::ParseError(
                    "Handler must be function call or block".to_string(),
                    self.previous().position,
                )),
            }
        }
    }

    fn parse_function_decl(&mut self) -> Result<FunctionDecl, CompilerError> {
        self.consume(TokenKind::Function, "Expected 'fn'")?;
        let name = self.consume_identifier("Expected function name")?;

        self.consume(TokenKind::LeftParen, "Expected '(' after function name")?;

        let mut parameters = Vec::new();

        while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
            if self.check(&TokenKind::Comma) {
                self.advance();
                continue;
            }

            let param_name = self.consume_parameter_name()?;
            let param_type = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };

            parameters.push(Parameter {
                name: param_name,
                param_type,
            });
        }

        self.consume(TokenKind::RightParen, "Expected ')' after parameters")?;

        let return_type = if self.check(&TokenKind::Arrow)
            || (self.check(&TokenKind::Minus)
                && self
                    .peek_next()
                    .is_some_and(|t| matches!(t.kind, TokenKind::Greater)))
        {
            // Handle both '=>' and '->' arrows
            if self.check(&TokenKind::Arrow) {
                self.advance();
            } else {
                // Handle '->' as two separate tokens
                self.advance(); // consume '-'
                self.advance(); // consume '>'
            }
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(FunctionDecl {
            name,
            parameters,
            return_type,
            body,
        })
    }

    fn parse_type(&mut self) -> Result<Type, CompilerError> {
        match &self.peek().kind {
            TokenKind::Identifier(name) => {
                let type_name = name.clone();
                self.advance();
                match type_name.as_str() {
                    "bool" => Ok(Type::Bool),
                    "int" => Ok(Type::Int),
                    "string" => Ok(Type::String),
                    "room" => Ok(Type::Room),
                    "object" => Ok(Type::Object),
                    _ => Err(CompilerError::ParseError(
                        format!("Unknown type: {}", type_name),
                        self.previous().position,
                    )),
                }
            }
            _ => {
                let token = self.peek();
                Err(CompilerError::ExpectedToken(
                    "type name".to_string(),
                    format!("{:?}", token.kind),
                    token.position,
                ))
            }
        }
    }

    fn parse_init_decl(&mut self) -> Result<InitDecl, CompilerError> {
        self.consume(TokenKind::Init, "Expected 'init'")?;
        let body = self.parse_block()?;

        Ok(InitDecl { body })
    }

    fn parse_block(&mut self) -> Result<BlockStmt, CompilerError> {
        self.consume(TokenKind::LeftBrace, "Expected '{'")?;

        let mut statements = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Skip newlines
            if self.check(&TokenKind::Newline) {
                self.advance();
                continue;
            }

            statements.push(self.parse_statement()?);
        }

        self.consume(TokenKind::RightBrace, "Expected '}'")?;

        Ok(BlockStmt { statements })
    }

    fn parse_statement(&mut self) -> Result<Stmt, CompilerError> {
        match &self.peek().kind {
            TokenKind::Let => self.parse_var_decl(false),
            TokenKind::Var => self.parse_var_decl(true),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::LeftBrace => Ok(Stmt::Block(self.parse_block()?)),
            _ => {
                // Check if this is an assignment or expression
                let checkpoint = self.current;

                // Try to parse as assignment
                if let Ok(target) = self.parse_expression() {
                    if self.check(&TokenKind::Equal) {
                        self.advance(); // consume '='
                        let value = self.parse_expression()?;
                        self.consume_semicolon_optional();
                        return Ok(Stmt::Assignment(AssignmentStmt { target, value }));
                    }
                }

                // Backtrack and parse as expression statement
                self.current = checkpoint;
                let expr = self.parse_expression()?;
                self.consume_semicolon_optional();
                Ok(Stmt::Expression(expr))
            }
        }
    }

    fn parse_var_decl(&mut self, mutable: bool) -> Result<Stmt, CompilerError> {
        if mutable {
            self.consume(TokenKind::Var, "Expected 'var'")?;
        } else {
            self.consume(TokenKind::Let, "Expected 'let'")?;
        }

        let name = self.consume_identifier("Expected variable name")?;

        let var_type = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let initializer = if self.check(&TokenKind::Equal) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume_semicolon_optional();

        Ok(Stmt::VarDecl(VarDeclStmt {
            name,
            mutable,
            var_type,
            initializer,
        }))
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, CompilerError> {
        self.consume(TokenKind::If, "Expected 'if'")?;
        let condition = self.parse_expression()?;
        let then_branch = Box::new(self.parse_statement()?);

        let else_branch = if self.check(&TokenKind::Else) {
            self.advance();
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        Ok(Stmt::If(IfStmt {
            condition,
            then_branch,
            else_branch,
        }))
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, CompilerError> {
        self.consume(TokenKind::While, "Expected 'while'")?;
        let condition = self.parse_expression()?;
        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::While(WhileStmt { condition, body }))
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, CompilerError> {
        self.consume(TokenKind::For, "Expected 'for'")?;
        let variable = self.consume_identifier("Expected loop variable name")?;
        // Skip 'in' keyword - simplified for now
        self.advance();
        let iterable = self.parse_expression()?;
        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::For(ForStmt {
            variable,
            iterable,
            body,
        }))
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, CompilerError> {
        self.consume(TokenKind::Return, "Expected 'return'")?;

        let value = if self.check(&TokenKind::Semicolon)
            || self.check(&TokenKind::Newline)
            || self.check(&TokenKind::RightBrace)
        {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.consume_semicolon_optional();

        Ok(Stmt::Return(value))
    }

    fn parse_expression(&mut self) -> Result<Expr, CompilerError> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Result<Expr, CompilerError> {
        let expr = self.parse_logical_or()?;

        if self.check(&TokenKind::Question) {
            self.advance();
            let true_expr = Box::new(self.parse_expression()?);
            self.consume(TokenKind::Colon, "Expected ':' in ternary expression")?;
            let false_expr = Box::new(self.parse_expression()?);

            Ok(Expr::Ternary {
                condition: Box::new(expr),
                true_expr,
                false_expr,
            })
        } else {
            Ok(expr)
        }
    }

    fn parse_logical_or(&mut self) -> Result<Expr, CompilerError> {
        let mut expr = self.parse_logical_and()?;

        while self.check(&TokenKind::Or) {
            self.advance();
            // Skip newlines after operators
            self.skip_newlines();
            let right = self.parse_logical_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator: BinaryOp::Or,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, CompilerError> {
        let mut expr = self.parse_equality()?;

        while self.check(&TokenKind::And) {
            self.advance();
            // Skip newlines after operators
            self.skip_newlines();
            let right = self.parse_equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator: BinaryOp::And,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, CompilerError> {
        let mut expr = self.parse_comparison()?;

        while self.match_token(&[TokenKind::EqualEqual, TokenKind::NotEqual]) {
            let operator = match self.previous().kind {
                TokenKind::EqualEqual => BinaryOp::Equal,
                TokenKind::NotEqual => BinaryOp::NotEqual,
                _ => unreachable!(),
            };
            let right = self.parse_comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, CompilerError> {
        let mut expr = self.parse_term()?;

        while self.match_token(&[
            TokenKind::Greater,
            TokenKind::GreaterEqual,
            TokenKind::Less,
            TokenKind::LessEqual,
        ]) {
            let operator = match self.previous().kind {
                TokenKind::Greater => BinaryOp::Greater,
                TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
                TokenKind::Less => BinaryOp::Less,
                TokenKind::LessEqual => BinaryOp::LessEqual,
                _ => unreachable!(),
            };
            let right = self.parse_term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, CompilerError> {
        let mut expr = self.parse_factor()?;

        while self.match_token(&[TokenKind::Minus, TokenKind::Plus]) {
            let operator = match self.previous().kind {
                TokenKind::Minus => BinaryOp::Subtract,
                TokenKind::Plus => BinaryOp::Add,
                _ => unreachable!(),
            };
            let right = self.parse_factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, CompilerError> {
        let mut expr = self.parse_unary()?;

        while self.match_token(&[TokenKind::Slash, TokenKind::Star, TokenKind::Percent]) {
            let operator = match self.previous().kind {
                TokenKind::Slash => BinaryOp::Divide,
                TokenKind::Star => BinaryOp::Multiply,
                TokenKind::Percent => BinaryOp::Modulo,
                _ => unreachable!(),
            };
            let right = self.parse_unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, CompilerError> {
        if self.match_token(&[TokenKind::Not, TokenKind::Minus]) {
            let operator = match self.previous().kind {
                TokenKind::Not => UnaryOp::Not,
                TokenKind::Minus => UnaryOp::Minus,
                _ => unreachable!(),
            };
            let operand = Box::new(self.parse_unary()?);
            Ok(Expr::Unary { operator, operand })
        } else {
            self.parse_call()
        }
    }

    fn parse_call(&mut self) -> Result<Expr, CompilerError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&TokenKind::LeftParen) {
                self.advance();
                let mut arguments = Vec::new();

                while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                        continue;
                    }
                    arguments.push(self.parse_expression()?);
                }

                self.consume(TokenKind::RightParen, "Expected ')' after arguments")?;

                // Convert identifier or property access to function call
                match expr {
                    Expr::Identifier(name) => {
                        expr = Expr::FunctionCall { name, arguments };
                    }
                    Expr::PropertyAccess { object, property } => {
                        // Convert to method call with proper object context
                        expr = Expr::MethodCall {
                            object,
                            method: property,
                            arguments,
                        };
                    }
                    _ => {
                        return Err(CompilerError::ParseError(
                            "Only identifiers and properties can be called as functions"
                                .to_string(),
                            self.previous().position,
                        ));
                    }
                }
            } else if self.check(&TokenKind::Dot) {
                self.advance();
                let property = self.consume_property_name()?;
                expr = Expr::PropertyAccess {
                    object: Box::new(expr),
                    property,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, CompilerError> {
        match &self.peek().kind {
            TokenKind::True => {
                self.advance();
                Ok(Expr::Boolean(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Boolean(false))
            }
            TokenKind::IntegerLiteral(val) => {
                let value = *val;
                self.advance();
                Ok(Expr::Integer(value))
            }
            TokenKind::StringLiteral(val) => {
                let value = val.clone();
                self.advance();
                Ok(Expr::String(value))
            }
            TokenKind::Identifier(name) => {
                let identifier = name.clone();
                self.advance();
                Ok(Expr::Identifier(identifier))
            }
            TokenKind::Parameter(param) => {
                let parameter = param.clone();
                self.advance();
                Ok(Expr::Parameter(parameter))
            }
            TokenKind::Print => {
                // Handle print as a function call
                self.advance();
                Ok(Expr::Identifier("print".to_string()))
            }
            TokenKind::Move => {
                // Handle move as a function call
                self.advance();
                Ok(Expr::Identifier("move".to_string()))
            }
            TokenKind::Player => {
                // Handle player as an identifier
                self.advance();
                Ok(Expr::Identifier("player".to_string()))
            }
            TokenKind::Location => {
                // Handle location as an identifier
                self.advance();
                Ok(Expr::Identifier("location".to_string()))
            }
            TokenKind::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume(TokenKind::RightParen, "Expected ')' after expression")?;
                Ok(expr)
            }
            TokenKind::LeftBracket => {
                self.advance();
                let mut elements = Vec::new();

                while !self.check(&TokenKind::RightBracket) && !self.is_at_end() {
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                        continue;
                    }
                    elements.push(self.parse_expression()?);
                }

                self.consume(TokenKind::RightBracket, "Expected ']' after array elements")?;
                Ok(Expr::Array(elements))
            }
            _ => {
                let token = self.peek();
                Err(CompilerError::ExpectedToken(
                    "expression".to_string(),
                    format!("{:?}", token.kind),
                    token.position,
                ))
            }
        }
    }

    // Helper methods
    fn match_token(&mut self, types: &[TokenKind]) -> bool {
        for token_type in types {
            if self.check(token_type) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check(&self, token_type: &TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(token_type)
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || matches!(self.peek().kind, TokenKind::EOF)
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.current).unwrap_or(&Token {
            kind: TokenKind::EOF,
            position: 0,
            line: 1,
            column: 1,
        })
    }

    fn peek_next(&self) -> Option<&Token> {
        self.tokens.get(self.current + 1)
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn consume(&mut self, token_type: TokenKind, _message: &str) -> Result<(), CompilerError> {
        if self.check(&token_type) {
            self.advance();
            Ok(())
        } else {
            let token = self.peek();
            Err(CompilerError::ExpectedToken(
                format!("{:?}", token_type),
                format!("{:?}", token.kind),
                token.position,
            ))
        }
    }

    fn consume_identifier(&mut self, _message: &str) -> Result<String, CompilerError> {
        if let TokenKind::Identifier(name) = &self.peek().kind {
            let identifier = name.clone();
            self.advance();
            Ok(identifier)
        } else {
            let token = self.peek();
            Err(CompilerError::ExpectedToken(
                "identifier".to_string(),
                format!("{:?}", token.kind),
                token.position,
            ))
        }
    }

    fn consume_string(&mut self, _message: &str) -> Result<String, CompilerError> {
        if let TokenKind::StringLiteral(value) = &self.peek().kind {
            let string = value.clone();
            self.advance();
            Ok(string)
        } else {
            let token = self.peek();
            Err(CompilerError::ExpectedToken(
                "string literal".to_string(),
                format!("{:?}", token.kind),
                token.position,
            ))
        }
    }

    fn consume_semicolon_optional(&mut self) {
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    fn consume_parameter_name(&mut self) -> Result<String, CompilerError> {
        // Allow both identifiers and keywords as parameter names
        match &self.peek().kind {
            TokenKind::Identifier(name) => {
                let param = name.clone();
                self.advance();
                Ok(param)
            }
            TokenKind::Location => {
                self.advance();
                Ok("location".to_string())
            }
            TokenKind::Object => {
                self.advance();
                Ok("object".to_string())
            }
            // Add other keywords that might be used as parameter names
            _ => {
                let token = self.peek();
                Err(CompilerError::ExpectedToken(
                    "parameter name".to_string(),
                    format!("{:?}", token.kind),
                    token.position,
                ))
            }
        }
    }

    fn consume_property_name(&mut self) -> Result<String, CompilerError> {
        // Allow both identifiers and keywords as property names
        match &self.peek().kind {
            TokenKind::Identifier(name) => {
                let property = name.clone();
                self.advance();
                Ok(property)
            }
            TokenKind::Location => {
                self.advance();
                Ok("location".to_string())
            }
            TokenKind::Desc => {
                self.advance();
                Ok("desc".to_string())
            }
            TokenKind::Names => {
                self.advance();
                Ok("names".to_string())
            }
            TokenKind::OnEnter => {
                self.advance();
                Ok("on_enter".to_string())
            }
            TokenKind::OnExit => {
                self.advance();
                Ok("on_exit".to_string())
            }
            TokenKind::OnLook => {
                self.advance();
                Ok("on_look".to_string())
            }
            TokenKind::Exits => {
                self.advance();
                Ok("exits".to_string())
            }
            TokenKind::Contains => {
                self.advance();
                Ok("contains".to_string())
            }
            TokenKind::Properties => {
                self.advance();
                Ok("properties".to_string())
            }
            // Add other keywords as needed
            _ => {
                let token = self.peek();
                Err(CompilerError::ExpectedToken(
                    "property name".to_string(),
                    format!("{:?}", token.kind),
                    token.position,
                ))
            }
        }
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
