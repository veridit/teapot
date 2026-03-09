//! Token module - defines the token types used in expressions

/// Token represents a value in a cell or an expression
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Empty cell or value
    Empty,
    /// Integer value
    Integer(i64),
    /// Floating point value
    Float(f64),
    /// String value
    String(String),
    /// Error value with message
    Error(String),
    /// Reference to another cell
    Location([usize; 3]),
    /// Identifier (function or variable name)
    Identifier(String),
    /// Label identifier (cell label)
    LabelIdentifier(String),
    /// Operator
    Operator(String),
}

impl Token {
    /// Returns true if the token is empty
    pub fn is_empty(&self) -> bool {
        matches!(self, Token::Empty)
    }
    
    /// Returns true if the token is an error
    pub fn is_error(&self) -> bool {
        matches!(self, Token::Error(_))
    }
    
    /// Returns the error message if this is an error token
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Token::Error(msg) => Some(msg),
            _ => None,
        }
    }
}
