use crate::eval;
use crate::sheet::Sheet;
use crate::token::{Operator, Token};

const MAX_ARGC: usize = 16;

/// Context for expression evaluation — replaces C globals upd_sheet, upd_x, etc.
pub struct EvalContext<'a> {
    pub sheet: &'a mut Sheet,
    pub x: usize,
    pub y: usize,
    pub z: usize,
    pub max_eval: i32,
}

/// Parse and evaluate a token sequence
pub fn eval_tokens(tokens: &[Token], ctx: &mut EvalContext) -> Token {
    if tokens.is_empty() {
        return Token::Empty;
    }
    let mut i = 0;
    let result = term(tokens, &mut i, ctx);
    if result.is_error() {
        return result;
    }
    if i < tokens.len() {
        return Token::Error("parse error after term".to_string());
    }
    result
}

/// Parse and evaluate a relational term (lowest precedence)
fn term(tokens: &[Token], i: &mut usize, ctx: &mut EvalContext) -> Token {
    let mut left = factor(tokens, i, ctx);
    if left.is_error() {
        return left;
    }

    while *i < tokens.len() {
        let op = match &tokens[*i] {
            Token::Operator(op @ (Operator::Lt | Operator::Le | Operator::Ge |
                                   Operator::Gt | Operator::Equal | Operator::AboutEqual |
                                   Operator::NotEqual)) => *op,
            _ => break,
        };
        *i += 1;
        let right = factor(tokens, i, ctx);
        let result = match op {
            Operator::Lt => eval::lt(&left, &right),
            Operator::Le => eval::le(&left, &right),
            Operator::Ge => eval::ge(&left, &right),
            Operator::Gt => eval::gt(&left, &right),
            Operator::Equal => eval::eq(&left, &right),
            Operator::AboutEqual => eval::about_eq(&left, &right),
            Operator::NotEqual => eval::ne(&left, &right),
            _ => unreachable!(),
        };
        if result.is_error() {
            return result;
        }
        left = result;
    }
    left
}

/// Parse and evaluate an additive factor
fn factor(tokens: &[Token], i: &mut usize, ctx: &mut EvalContext) -> Token {
    let mut left = piterm(tokens, i, ctx);
    if left.is_error() {
        return left;
    }

    while *i < tokens.len() {
        let op = match &tokens[*i] {
            Token::Operator(op @ (Operator::Plus | Operator::Minus)) => *op,
            _ => break,
        };
        *i += 1;
        let right = piterm(tokens, i, ctx);
        let result = match op {
            Operator::Plus => eval::add(&left, &right),
            Operator::Minus => eval::sub(&left, &right),
            _ => unreachable!(),
        };
        if result.is_error() {
            return result;
        }
        left = result;
    }
    left
}

/// Parse and evaluate a multiplicative/divisive/modulo term
fn piterm(tokens: &[Token], i: &mut usize, ctx: &mut EvalContext) -> Token {
    let mut left = powterm(tokens, i, ctx);
    if left.is_error() {
        return left;
    }

    while *i < tokens.len() {
        let op = match &tokens[*i] {
            Token::Operator(op @ (Operator::Mul | Operator::Div | Operator::Mod)) => *op,
            _ => break,
        };
        *i += 1;
        let right = powterm(tokens, i, ctx);
        let result = match op {
            Operator::Mul => eval::mul(&left, &right),
            Operator::Div => eval::div(&left, &right),
            Operator::Mod => eval::modulo(&left, &right),
            _ => unreachable!(),
        };
        if result.is_error() {
            return result;
        }
        left = result;
    }
    left
}

/// Parse and evaluate a power term
fn powterm(tokens: &[Token], i: &mut usize, ctx: &mut EvalContext) -> Token {
    let mut left = primary(tokens, i, ctx);
    if left.is_error() {
        return left;
    }

    while *i < tokens.len() {
        match &tokens[*i] {
            Token::Operator(Operator::Pow) => {}
            _ => break,
        }
        *i += 1;
        let right = primary(tokens, i, ctx);
        let result = eval::pow(&left, &right);
        if result.is_error() {
            return result;
        }
        left = result;
    }
    left
}

/// Parse and evaluate a primary term (highest precedence)
fn primary(tokens: &[Token], i: &mut usize, ctx: &mut EvalContext) -> Token {
    if *i >= tokens.len() {
        return Token::Error("missing operator".to_string());
    }

    match &tokens[*i] {
        // Literal values
        Token::Integer(_) | Token::Float(_) | Token::String(_) => {
            let tok = tokens[*i].clone();
            *i += 1;
            tok
        }

        // Parenthesized expression or unary minus
        Token::Operator(Operator::OpenParen) => {
            *i += 1;
            let result = term(tokens, i, ctx);
            if result.is_error() {
                return result;
            }
            if *i < tokens.len() && tokens[*i] == Token::Operator(Operator::CloseParen) {
                *i += 1;
                result
            } else {
                Token::Error(") expected".to_string())
            }
        }

        Token::Operator(Operator::Minus) => {
            *i += 1;
            let operand = primary(tokens, i, ctx);
            eval::neg(&operand)
        }

        Token::Operator(_) => {
            Token::Error("value expected".to_string())
        }

        // Label identifier — look up in sheet
        Token::LabelIdentifier(name) => {
            let name = name.clone();
            *i += 1;
            ctx.sheet.findlabel(&name)
        }

        // Function call
        Token::Identifier(name) => {
            let name = name.clone();
            *i += 1;

            // Expect opening paren
            if *i >= tokens.len() || tokens[*i] != Token::Operator(Operator::OpenParen) {
                return Token::Error("( expected".to_string());
            }
            *i += 1; // skip '('

            let mut args = Vec::with_capacity(MAX_ARGC);

            // Check for immediate closing paren (no arguments)
            if *i < tokens.len() && tokens[*i] == Token::Operator(Operator::CloseParen) {
                *i += 1;
                return crate::functions::call_function(&name, &args, ctx);
            }

            // Parse first argument (may be empty if comma follows)
            if *i < tokens.len() && tokens[*i] == Token::Operator(Operator::Comma) {
                args.push(Token::Empty);
            } else {
                let arg = term(tokens, i, ctx);
                if arg.is_error() {
                    return arg;
                }
                args.push(arg);
            }

            // Parse remaining arguments separated by commas
            while *i < tokens.len() && tokens[*i] == Token::Operator(Operator::Comma) {
                *i += 1;
                if args.len() >= MAX_ARGC {
                    return Token::Error("too many arguments".to_string());
                }
                if *i < tokens.len()
                    && matches!(&tokens[*i], Token::Operator(Operator::Comma) | Token::Operator(Operator::CloseParen))
                {
                    args.push(Token::Empty);
                } else {
                    let arg = term(tokens, i, ctx);
                    if arg.is_error() {
                        return arg;
                    }
                    args.push(arg);
                }
            }

            // Expect closing paren
            if *i < tokens.len() && tokens[*i] == Token::Operator(Operator::CloseParen) {
                *i += 1;
                crate::functions::call_function(&name, &args, ctx)
            } else {
                Token::Error(") expected".to_string())
            }
        }

        _ => Token::Error("value expected".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner;

    fn eval_expr(input: &str) -> Token {
        let tokens = scanner::scan(input).unwrap();
        let mut sheet = Sheet::new();
        let mut ctx = EvalContext {
            sheet: &mut sheet,
            x: 0, y: 0, z: 0,
            max_eval: 256,
        };
        eval_tokens(&tokens, &mut ctx)
    }

    #[test]
    fn test_simple_addition() {
        assert_eq!(eval_expr("1+2"), Token::Integer(3));
    }

    #[test]
    fn test_operator_precedence() {
        assert_eq!(eval_expr("2+3*4"), Token::Integer(14));
    }

    #[test]
    fn test_parentheses() {
        assert_eq!(eval_expr("(2+3)*4"), Token::Integer(20));
    }

    #[test]
    fn test_power() {
        assert_eq!(eval_expr("2^10"), Token::Integer(1024));
    }

    #[test]
    fn test_unary_minus() {
        assert_eq!(eval_expr("-5"), Token::Integer(-5));
        assert_eq!(eval_expr("-5+3"), Token::Integer(-2));
    }

    #[test]
    fn test_float_expression() {
        assert_eq!(eval_expr("1.5+2.5"), Token::Float(4.0));
    }

    #[test]
    fn test_mixed_types() {
        assert_eq!(eval_expr("1+2.0"), Token::Float(3.0));
    }

    #[test]
    fn test_comparison() {
        assert_eq!(eval_expr("1<2"), Token::Integer(1));
        assert_eq!(eval_expr("2<1"), Token::Integer(0));
    }

    #[test]
    fn test_complex_expression() {
        assert_eq!(eval_expr("(1+2)*(3+4)"), Token::Integer(21));
    }

    #[test]
    fn test_string_literal() {
        assert_eq!(eval_expr("\"hello\""), Token::String("hello".to_string()));
    }

    #[test]
    fn test_division() {
        assert_eq!(eval_expr("10/3"), Token::Integer(3));
        assert_eq!(eval_expr("10.0/3.0"), Token::Float(10.0 / 3.0));
    }

    #[test]
    fn test_modulo() {
        assert_eq!(eval_expr("10%3"), Token::Integer(1));
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(eval_expr(""), Token::Empty);
    }

    #[test]
    fn test_parse_error() {
        assert!(eval_expr("1+").is_error());
        assert!(eval_expr("(1+2").is_error());
    }

    #[test]
    fn test_function_call_simple() {
        assert_eq!(eval_expr("sin(0)"), Token::Float(0.0));
    }

    #[test]
    fn test_function_call_nested() {
        let result = eval_expr("sin(asin(0.5))");
        match result {
            Token::Float(f) => assert!((f - 0.5).abs() < 1e-10),
            _ => panic!("expected Float, got {:?}", result),
        }
    }

    #[test]
    fn test_function_call_multi_arg() {
        assert_eq!(eval_expr("substr(\"hello\",1,3)"), Token::String("ell".to_string()));
    }

    #[test]
    fn test_function_call_zero_args() {
        match eval_expr("time()") {
            Token::Integer(t) => assert!(t > 0),
            _ => panic!("expected Integer from time()"),
        }
    }

    #[test]
    fn test_function_in_arithmetic() {
        assert_eq!(eval_expr("abs(-3)+abs(-4)"), Token::Integer(7));
    }

    #[test]
    fn test_nested_parens_deep() {
        assert_eq!(eval_expr("((((1+2))))"), Token::Integer(3));
    }

    #[test]
    fn test_about_equal_operator() {
        assert_eq!(eval_expr("1.0~=1.0"), Token::Integer(1));
    }

    #[test]
    fn test_function_call_error() {
        assert!(eval_expr("sin()").is_error());
    }
}
