use crate::token::{Operator, Token};
use anyhow::{bail, Result};

/// Known function names — identifiers matching these become Token::Identifier,
/// everything else becomes Token::LabelIdentifier
const FUNCTION_NAMES: &[&str] = &[
    "@", "&", "x", "y", "z", "eval", "error", "string", "sum", "n",
    "int", "frac", "len", "min", "max", "abs", "$", "float", "strftime",
    "clock", "poly", "e", "log", "sin", "cos", "tan", "sinh", "cosh",
    "tanh", "asin", "acos", "atan", "arsinh", "arcosh", "artanh",
    "deg2rad", "rad2deg", "rnd", "substr", "strptime", "time",
];

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '@' || c == '&' || c == '.' || c == '$'
}

fn is_ident_cont(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}

fn is_function_name(name: &str) -> bool {
    FUNCTION_NAMES.contains(&name)
}

/// Scan a string into a sequence of tokens
pub fn scan(input: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let mut pos = 0;

    // Skip leading spaces
    while pos < bytes.len() && bytes[pos] == b' ' {
        pos += 1;
    }

    while pos < bytes.len() {
        // Skip spaces between tokens
        while pos < bytes.len() && bytes[pos] == b' ' {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }

        let start = pos;

        // Try quoted string
        if let Some((tok, new_pos)) = scan_string(input, pos) {
            tokens.push(tok);
            pos = new_pos;
            continue;
        }

        // Try operator
        if let Some((tok, new_pos)) = scan_operator(input, pos) {
            tokens.push(tok);
            pos = new_pos;
            continue;
        }

        // Try integer (must not be followed by '.' or 'e' — those are floats)
        if let Some((tok, new_pos)) = scan_integer(input, pos) {
            tokens.push(tok);
            pos = new_pos;
            continue;
        }

        // Try float
        if let Some((tok, new_pos)) = scan_float(input, pos) {
            tokens.push(tok);
            pos = new_pos;
            continue;
        }

        // Try identifier
        if let Some((tok, new_pos)) = scan_identifier(input, pos) {
            tokens.push(tok);
            pos = new_pos;
            continue;
        }

        // No token matched — error
        if pos == start {
            bail!("unexpected character '{}' at position {}", input[pos..].chars().next().unwrap_or('?'), pos);
        }
    }

    Ok(tokens)
}

fn scan_string(input: &str, pos: usize) -> Option<(Token, usize)> {
    let bytes = input.as_bytes();
    if bytes[pos] != b'"' {
        return None;
    }

    let mut i = pos + 1;
    let mut s = String::new();

    while i < bytes.len() && bytes[i] != b'"' {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            s.push(bytes[i + 1] as char);
            i += 2;
        } else {
            s.push(bytes[i] as char);
            i += 1;
        }
    }

    if i >= bytes.len() {
        // Unterminated string
        return None;
    }

    // Skip closing quote
    i += 1;
    Some((Token::String(s), i))
}

fn scan_integer(input: &str, pos: usize) -> Option<(Token, usize)> {
    let bytes = input.as_bytes();
    if !bytes[pos].is_ascii_digit() {
        return None;
    }

    let mut i = pos;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }

    // If followed by '.' or 'e', it's a float, not an integer
    if i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b'e' || bytes[i] == b'E') {
        return None;
    }

    let s = &input[pos..i];
    let val: i64 = s.parse().ok()?;
    Some((Token::Integer(val), i))
}

fn scan_float(input: &str, pos: usize) -> Option<(Token, usize)> {
    let bytes = input.as_bytes();

    // Must start with digit or '.'
    if !bytes[pos].is_ascii_digit() && bytes[pos] != b'.' {
        return None;
    }

    // Use strtod-like parsing: find the longest prefix that parses as f64
    let rest = &input[pos..];
    // Find how far digits, '.', 'e', '+', '-' extend
    let mut i = 0;
    let rest_bytes = rest.as_bytes();

    // Integer part
    while i < rest_bytes.len() && rest_bytes[i].is_ascii_digit() {
        i += 1;
    }

    // Fractional part
    if i < rest_bytes.len() && rest_bytes[i] == b'.' {
        i += 1;
        while i < rest_bytes.len() && rest_bytes[i].is_ascii_digit() {
            i += 1;
        }
    }

    // Exponent part
    if i < rest_bytes.len() && (rest_bytes[i] == b'e' || rest_bytes[i] == b'E') {
        i += 1;
        if i < rest_bytes.len() && (rest_bytes[i] == b'+' || rest_bytes[i] == b'-') {
            i += 1;
        }
        while i < rest_bytes.len() && rest_bytes[i].is_ascii_digit() {
            i += 1;
        }
    }

    if i == 0 {
        return None;
    }

    let s = &rest[..i];
    let val: f64 = s.parse().ok()?;

    if val.is_infinite() || val.is_nan() {
        return None;
    }

    Some((Token::Float(val), pos + i))
}

fn scan_operator(input: &str, pos: usize) -> Option<(Token, usize)> {
    let bytes = input.as_bytes();
    let c = bytes[pos];
    let next = if pos + 1 < bytes.len() { Some(bytes[pos + 1]) } else { None };

    let (op, len) = match c {
        b'+' => (Operator::Plus, 1),
        b'-' => (Operator::Minus, 1),
        b'*' => (Operator::Mul, 1),
        b'/' => (Operator::Div, 1),
        b'%' => (Operator::Mod, 1),
        b'(' => (Operator::OpenParen, 1),
        b')' => (Operator::CloseParen, 1),
        b',' => (Operator::Comma, 1),
        b'^' => (Operator::Pow, 1),
        b'<' => {
            if next == Some(b'=') { (Operator::Le, 2) } else { (Operator::Lt, 1) }
        }
        b'>' => {
            if next == Some(b'=') { (Operator::Ge, 2) } else { (Operator::Gt, 1) }
        }
        b'=' => {
            if next == Some(b'=') { (Operator::Equal, 2) } else { return None; }
        }
        b'~' => {
            if next == Some(b'=') { (Operator::AboutEqual, 2) } else { return None; }
        }
        b'!' => {
            if next == Some(b'=') { (Operator::NotEqual, 2) } else { return None; }
        }
        _ => return None,
    };

    Some((Token::Operator(op), pos + len))
}

fn scan_identifier(input: &str, pos: usize) -> Option<(Token, usize)> {
    let c = input[pos..].chars().next()?;
    if !is_ident_start(c) {
        return None;
    }

    let mut i = pos + c.len_utf8();
    while i < input.len() {
        if let Some(c) = input[i..].chars().next() {
            if is_ident_cont(c) {
                i += c.len_utf8();
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let name = &input[pos..i];
    let tok = if is_function_name(name) {
        Token::Identifier(name.to_string())
    } else {
        Token::LabelIdentifier(name.to_string())
    };

    Some((tok, i))
}

/// Format a token sequence back into a string for display/serialization.
/// When `quote` is true, strings are quoted with backslash escaping.
pub fn print_tokens(tokens: &[Token], quote: bool, scientific: bool, precision: i32) -> String {
    let mut out = String::new();
    for token in tokens {
        match token {
            Token::Empty => {}
            Token::String(s) => {
                if quote {
                    out.push('"');
                    for c in s.chars() {
                        if c == '"' || c == '\\' {
                            out.push('\\');
                        }
                        out.push(c);
                    }
                    out.push('"');
                } else {
                    out.push_str(s);
                }
            }
            Token::Integer(i) => {
                out.push_str(&i.to_string());
            }
            Token::Float(f) => {
                if scientific {
                    let p = if precision < 0 { 13 } else { precision as usize };
                    out.push_str(&format!("{:.*e}", p, f));
                } else {
                    let p = if precision < 0 { 13 } else { precision as usize };
                    let s = format!("{:.*}", p, f);
                    // Trim trailing zeros after decimal point (unless precision was specified)
                    if precision < 0 && s.contains('.') {
                        let trimmed = s.trim_end_matches('0');
                        let trimmed = if trimmed.ends_with('.') { trimmed } else { trimmed };
                        out.push_str(trimmed);
                    } else {
                        out.push_str(&s);
                    }
                }
            }
            Token::Operator(op) => out.push_str(&op.to_string()),
            Token::Identifier(name) => out.push_str(name),
            Token::LabelIdentifier(name) => out.push_str(name),
            Token::Location(loc) => {
                out.push_str(&format!("&({},{},{})", loc[0], loc[1], loc[2]));
            }
            Token::Error(_) => out.push_str("ERROR"),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_integer() {
        let tokens = scan("42").unwrap();
        assert_eq!(tokens, vec![Token::Integer(42)]);
    }

    #[test]
    fn test_scan_float() {
        let tokens = scan("3.14").unwrap();
        assert_eq!(tokens, vec![Token::Float(3.14)]);
    }

    #[test]
    fn test_scan_float_exponent() {
        let tokens = scan("1e10").unwrap();
        assert_eq!(tokens, vec![Token::Float(1e10)]);
    }

    #[test]
    fn test_scan_string() {
        let tokens = scan("\"hello\"").unwrap();
        assert_eq!(tokens, vec![Token::String("hello".to_string())]);
    }

    #[test]
    fn test_scan_string_with_escapes() {
        let tokens = scan("\"he\\\"llo\"").unwrap();
        assert_eq!(tokens, vec![Token::String("he\"llo".to_string())]);
    }

    #[test]
    fn test_scan_simple_expression() {
        let tokens = scan("1+2*3").unwrap();
        assert_eq!(tokens, vec![
            Token::Integer(1),
            Token::Operator(Operator::Plus),
            Token::Integer(2),
            Token::Operator(Operator::Mul),
            Token::Integer(3),
        ]);
    }

    #[test]
    fn test_scan_expression_with_spaces() {
        let tokens = scan("1 + 2").unwrap();
        assert_eq!(tokens, vec![
            Token::Integer(1),
            Token::Operator(Operator::Plus),
            Token::Integer(2),
        ]);
    }

    #[test]
    fn test_scan_two_char_operators() {
        let tokens = scan("1<=2").unwrap();
        assert_eq!(tokens, vec![
            Token::Integer(1),
            Token::Operator(Operator::Le),
            Token::Integer(2),
        ]);

        let tokens = scan("1!=2").unwrap();
        assert_eq!(tokens, vec![
            Token::Integer(1),
            Token::Operator(Operator::NotEqual),
            Token::Integer(2),
        ]);
    }

    #[test]
    fn test_scan_function_identifier() {
        let tokens = scan("sin(3.14)").unwrap();
        assert_eq!(tokens, vec![
            Token::Identifier("sin".to_string()),
            Token::Operator(Operator::OpenParen),
            Token::Float(3.14),
            Token::Operator(Operator::CloseParen),
        ]);
    }

    #[test]
    fn test_scan_label_identifier() {
        let tokens = scan("myLabel").unwrap();
        assert_eq!(tokens, vec![Token::LabelIdentifier("myLabel".to_string())]);
    }

    #[test]
    fn test_scan_at_function() {
        let tokens = scan("@(1,2,0)").unwrap();
        assert_eq!(tokens, vec![
            Token::Identifier("@".to_string()),
            Token::Operator(Operator::OpenParen),
            Token::Integer(1),
            Token::Operator(Operator::Comma),
            Token::Integer(2),
            Token::Operator(Operator::Comma),
            Token::Integer(0),
            Token::Operator(Operator::CloseParen),
        ]);
    }

    #[test]
    fn test_scan_negative_number() {
        // Negative numbers are parsed as Minus + Integer
        let tokens = scan("-5").unwrap();
        assert_eq!(tokens, vec![
            Token::Operator(Operator::Minus),
            Token::Integer(5),
        ]);
    }

    #[test]
    fn test_scan_parens() {
        let tokens = scan("(1+2)*3").unwrap();
        assert_eq!(tokens, vec![
            Token::Operator(Operator::OpenParen),
            Token::Integer(1),
            Token::Operator(Operator::Plus),
            Token::Integer(2),
            Token::Operator(Operator::CloseParen),
            Token::Operator(Operator::Mul),
            Token::Integer(3),
        ]);
    }

    #[test]
    fn test_scan_empty() {
        let tokens = scan("").unwrap();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_print_tokens_roundtrip() {
        let tokens = scan("1+2*3").unwrap();
        let printed = print_tokens(&tokens, false, false, -1);
        assert_eq!(printed, "1+2*3");
    }

    #[test]
    fn test_print_tokens_string_quoted() {
        let s = print_tokens(&[Token::String("hello".to_string())], true, false, -1);
        assert_eq!(s, "\"hello\"");
    }
}
