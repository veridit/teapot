use std::fmt;

/// Operator types used in expressions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    Plus,
    Minus,
    Mul,
    Div,
    Mod,
    Pow,
    OpenParen,
    CloseParen,
    Comma,
    Lt,
    Le,
    Ge,
    Gt,
    Equal,
    AboutEqual,
    NotEqual,
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operator::Plus => write!(f, "+"),
            Operator::Minus => write!(f, "-"),
            Operator::Mul => write!(f, "*"),
            Operator::Div => write!(f, "/"),
            Operator::OpenParen => write!(f, "("),
            Operator::CloseParen => write!(f, ")"),
            Operator::Comma => write!(f, ","),
            Operator::Lt => write!(f, "<"),
            Operator::Le => write!(f, "<="),
            Operator::Ge => write!(f, ">="),
            Operator::Gt => write!(f, ">"),
            Operator::Equal => write!(f, "=="),
            Operator::AboutEqual => write!(f, "~="),
            Operator::NotEqual => write!(f, "!="),
            Operator::Pow => write!(f, "^"),
            Operator::Mod => write!(f, "%"),
        }
    }
}

/// Token represents a value in a cell or an expression
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Empty,
    Integer(i64),
    Float(f64),
    String(String),
    Error(String),
    Location([usize; 3]),
    /// Function identifier (known function name)
    Identifier(String),
    /// Label identifier (cell label reference)
    LabelIdentifier(String),
    Operator(Operator),
}

impl Token {
    pub fn is_empty(&self) -> bool {
        matches!(self, Token::Empty)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Token::Error(_))
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            Token::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Empty => Ok(()),
            Token::Integer(i) => write!(f, "{}", i),
            Token::Float(v) => write!(f, "{}", v),
            Token::String(s) => write!(f, "{}", s),
            Token::Error(_) => write!(f, "ERROR"),
            Token::Location(loc) => write!(f, "&({},{},{})", loc[0], loc[1], loc[2]),
            Token::Identifier(name) => write!(f, "{}", name),
            Token::LabelIdentifier(name) => write!(f, "{}", name),
            Token::Operator(op) => write!(f, "{}", op),
        }
    }
}
