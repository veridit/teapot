use crate::token::Token;

fn check_float(value: f64, op: &str) -> Token {
    if value.is_infinite() || value.is_nan() {
        Token::Error(format!("{}: float result out of range", op))
    } else {
        Token::Float(value)
    }
}

pub fn add(left: &Token, right: &Token) -> Token {
    match (left, right) {
        (Token::Error(_), _) => left.clone(),
        (_, Token::Error(_)) => right.clone(),
        (Token::Integer(l), Token::Integer(r)) => Token::Integer(l + r),
        (Token::String(l), Token::String(r)) => Token::String(format!("{}{}", l, r)),
        (Token::Empty, r @ (Token::Integer(_) | Token::Float(_) | Token::String(_) | Token::Empty)) => r.clone(),
        (l @ (Token::Integer(_) | Token::Float(_) | Token::String(_)), Token::Empty) => l.clone(),
        (Token::Integer(l), Token::Float(r)) => check_float(*l as f64 + r, "+"),
        (Token::Float(l), Token::Integer(r)) => check_float(l + *r as f64, "+"),
        (Token::Float(l), Token::Float(r)) => check_float(l + r, "+"),
        _ => Token::Error("wrong types for + operator".to_string()),
    }
}

pub fn sub(left: &Token, right: &Token) -> Token {
    match (left, right) {
        (Token::Error(_), _) => left.clone(),
        (_, Token::Error(_)) => right.clone(),
        (Token::Integer(l), Token::Integer(r)) => Token::Integer(l - r),
        (Token::Float(l), Token::Float(r)) => check_float(l - r, "-"),
        (Token::Empty, _) => neg(right),
        (Token::Integer(_) | Token::Float(_), Token::Empty) => left.clone(),
        (Token::Integer(l), Token::Float(r)) => check_float(*l as f64 - r, "-"),
        (Token::Float(l), Token::Integer(r)) => check_float(l - *r as f64, "-"),
        _ => Token::Error("wrong types for - operator".to_string()),
    }
}

pub fn mul(left: &Token, right: &Token) -> Token {
    match (left, right) {
        (Token::Error(_), _) => left.clone(),
        (_, Token::Error(_)) => right.clone(),
        (Token::Integer(l), Token::Integer(r)) => Token::Integer(l * r),
        (Token::Float(l), Token::Float(r)) => check_float(l * r, "*"),
        (Token::Empty, Token::Integer(_)) | (Token::Integer(_), Token::Empty) => Token::Integer(0),
        (Token::Empty, Token::Float(_)) | (Token::Float(_), Token::Empty) => Token::Float(0.0),
        (Token::Integer(l), Token::Float(r)) => check_float(*l as f64 * r, "*"),
        (Token::Float(l), Token::Integer(r)) => check_float(l * *r as f64, "*"),
        (Token::Empty, Token::Empty) => Token::Empty,
        _ => Token::Error("wrong types for * operator".to_string()),
    }
}

pub fn div(left: &Token, right: &Token) -> Token {
    match (left, right) {
        (Token::Error(_), _) => left.clone(),
        (_, Token::Error(_)) => right.clone(),
        (_, Token::Integer(0)) | (_, Token::Empty) => Token::Error("division by 0".to_string()),
        (_, Token::Float(r)) if *r == 0.0 => Token::Error("division by 0".to_string()),
        (Token::Integer(l), Token::Integer(r)) => Token::Integer(l / r),
        (Token::Float(l), Token::Float(r)) => check_float(l / r, "/"),
        (Token::Empty, Token::Integer(_)) => Token::Integer(0),
        (Token::Empty, Token::Float(_)) => Token::Float(0.0),
        (Token::Integer(l), Token::Float(r)) => check_float(*l as f64 / r, "/"),
        (Token::Float(l), Token::Integer(r)) => check_float(l / *r as f64, "/"),
        _ => Token::Error("wrong types for / operator".to_string()),
    }
}

pub fn modulo(left: &Token, right: &Token) -> Token {
    match (left, right) {
        (Token::Error(_), _) => left.clone(),
        (_, Token::Error(_)) => right.clone(),
        (_, Token::Integer(0)) | (_, Token::Empty) => Token::Error("modulo 0".to_string()),
        (_, Token::Float(r)) if *r == 0.0 => Token::Error("modulo 0".to_string()),
        (Token::Integer(l), Token::Integer(r)) => Token::Integer(l % r),
        (Token::Float(l), Token::Float(r)) => check_float(l.rem_euclid(*r), "%"),
        (Token::Empty, Token::Integer(_)) => Token::Integer(0),
        (Token::Empty, Token::Float(_)) => Token::Float(0.0),
        (Token::Integer(l), Token::Float(r)) => check_float((*l as f64).rem_euclid(*r), "%"),
        (Token::Float(l), Token::Integer(r)) => check_float(l.rem_euclid(*r as f64), "%"),
        _ => Token::Error("wrong types for % operator".to_string()),
    }
}

pub fn neg(token: &Token) -> Token {
    match token {
        Token::Error(_) => token.clone(),
        Token::Integer(i) => Token::Integer(-i),
        Token::Float(f) => Token::Float(-f),
        Token::Empty => Token::Empty,
        _ => Token::Error("wrong type for - operator".to_string()),
    }
}

pub fn pow(base: &Token, exponent: &Token) -> Token {
    if matches!(base, Token::Error(_)) { return base.clone(); }
    if matches!(exponent, Token::Error(_)) { return exponent.clone(); }

    let is_numeric = |t: &Token| matches!(t, Token::Integer(_) | Token::Float(_) | Token::Empty);
    if !is_numeric(base) || !is_numeric(exponent) {
        return Token::Error("wrong types for ^ operator".to_string());
    }

    // Integer path: both are int or empty, and exponent >= 0
    let try_int = match (base, exponent) {
        (Token::Integer(_) | Token::Empty, Token::Integer(y)) if *y >= 0 => true,
        (Token::Integer(_) | Token::Empty, Token::Empty) => true,
        _ => false,
    };

    if try_int {
        let x = match base { Token::Integer(v) => *v, _ => 0 };
        let y = match exponent { Token::Integer(v) => *v, _ => 0 };
        if x == 0 && y == 0 {
            return Token::Error("0^0 is not defined".to_string());
        } else if x == 0 {
            return Token::Integer(0);
        } else if y == 0 {
            return Token::Integer(1);
        } else {
            let mut result = x;
            for _ in 1..y {
                result *= x;
            }
            return Token::Integer(result);
        }
    }

    // Float path
    let x = match base {
        Token::Integer(v) => *v as f64,
        Token::Float(v) => *v,
        _ => 0.0,
    };
    let y = match exponent {
        Token::Integer(v) => *v as f64,
        Token::Float(v) => *v,
        _ => 0.0,
    };
    let result = x.powf(y);
    if result.is_infinite() || result.is_nan() {
        Token::Error("^ caused a domain error".to_string())
    } else {
        Token::Float(result)
    }
}

/// Coerce Empty to the zero-value of the other operand's type for comparisons
fn coerce_for_cmp<'a>(left: &'a Token, right: &'a Token) -> (Token, Token) {
    let l = if matches!(left, Token::Empty) {
        match right {
            Token::Integer(_) => Token::Integer(0),
            Token::Float(_) => Token::Float(0.0),
            Token::String(_) => Token::String(String::new()),
            _ => Token::Empty,
        }
    } else {
        left.clone()
    };
    let r = if matches!(right, Token::Empty) {
        match &l {
            Token::Integer(_) => Token::Integer(0),
            Token::Float(_) => Token::Float(0.0),
            Token::String(_) => Token::String(String::new()),
            _ => Token::Empty,
        }
    } else {
        right.clone()
    };
    (l, r)
}

fn cmp_tokens(left: &Token, right: &Token, int_cmp: fn(i64, i64) -> bool, float_cmp: fn(f64, f64) -> bool, str_cmp: fn(i32) -> bool) -> Token {
    if matches!(left, Token::Error(_)) { return left.clone(); }
    if matches!(right, Token::Error(_)) { return right.clone(); }
    let (l, r) = coerce_for_cmp(left, right);
    match (&l, &r) {
        (Token::Integer(a), Token::Integer(b)) => Token::Integer(int_cmp(*a, *b) as i64),
        (Token::Float(a), Token::Float(b)) => Token::Integer(float_cmp(*a, *b) as i64),
        (Token::Integer(a), Token::Float(b)) => Token::Integer(float_cmp(*a as f64, *b) as i64),
        (Token::Float(a), Token::Integer(b)) => Token::Integer(float_cmp(*a, *b as f64) as i64),
        (Token::String(a), Token::String(b)) => {
            let ord = a.cmp(b) as i32;
            Token::Integer(str_cmp(ord) as i64)
        }
        (Token::Empty, Token::Empty) => Token::Integer(int_cmp(0, 0) as i64),
        _ => Token::Error("type mismatch for relational operator".to_string()),
    }
}

pub fn lt(left: &Token, right: &Token) -> Token {
    cmp_tokens(left, right, |a, b| a < b, |a, b| a < b, |o| o < 0)
}

pub fn le(left: &Token, right: &Token) -> Token {
    cmp_tokens(left, right, |a, b| a <= b, |a, b| a <= b, |o| o <= 0)
}

pub fn ge(left: &Token, right: &Token) -> Token {
    cmp_tokens(left, right, |a, b| a >= b, |a, b| a >= b, |o| o >= 0)
}

pub fn gt(left: &Token, right: &Token) -> Token {
    cmp_tokens(left, right, |a, b| a > b, |a, b| a > b, |o| o > 0)
}

pub fn eq(left: &Token, right: &Token) -> Token {
    if matches!(left, Token::Error(_)) { return left.clone(); }
    if matches!(right, Token::Error(_)) { return right.clone(); }
    let (l, r) = coerce_for_cmp(left, right);
    match (&l, &r) {
        (Token::Integer(a), Token::Integer(b)) => Token::Integer((a == b) as i64),
        (Token::Float(a), Token::Float(b)) => Token::Integer((a == b) as i64),
        (Token::Integer(a), Token::Float(b)) => Token::Integer((*a as f64 == *b) as i64),
        (Token::Float(a), Token::Integer(b)) => Token::Integer((*a == *b as f64) as i64),
        (Token::String(a), Token::String(b)) => Token::Integer((a == b) as i64),
        (Token::Empty, Token::Empty) => Token::Integer(1),
        _ if std::mem::discriminant(&l) != std::mem::discriminant(&r) => Token::Integer(0),
        _ => Token::Error("type mismatch for relational operator".to_string()),
    }
}

pub fn about_eq(left: &Token, right: &Token) -> Token {
    if matches!(left, Token::Error(_)) { return left.clone(); }
    if matches!(right, Token::Error(_)) { return right.clone(); }
    let (l, r) = coerce_for_cmp(left, right);
    match (&l, &r) {
        (Token::Float(a), Token::Float(b)) => Token::Integer(((a - b).abs() <= f64::EPSILON) as i64),
        _ => Token::Error("type mismatch for relational operator".to_string()),
    }
}

pub fn ne(left: &Token, right: &Token) -> Token {
    if matches!(left, Token::Error(_)) { return left.clone(); }
    if matches!(right, Token::Error(_)) { return right.clone(); }
    let (l, r) = coerce_for_cmp(left, right);
    match (&l, &r) {
        (Token::Integer(a), Token::Integer(b)) => Token::Integer((a != b) as i64),
        (Token::Float(a), Token::Float(b)) => Token::Integer((a != b) as i64),
        (Token::Integer(a), Token::Float(b)) => Token::Integer((*a as f64 != *b) as i64),
        (Token::Float(a), Token::Integer(b)) => Token::Integer((*a != *b as f64) as i64),
        (Token::String(a), Token::String(b)) => Token::Integer((a != b) as i64),
        (Token::Empty, Token::Empty) => Token::Integer(0),
        _ if std::mem::discriminant(&l) != std::mem::discriminant(&r) => Token::Integer(1),
        _ => Token::Error("type mismatch for relational operator".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_integers() {
        assert_eq!(add(&Token::Integer(1), &Token::Integer(2)), Token::Integer(3));
    }

    #[test]
    fn test_add_floats() {
        assert_eq!(add(&Token::Float(1.5), &Token::Float(2.5)), Token::Float(4.0));
    }

    #[test]
    fn test_add_int_float() {
        assert_eq!(add(&Token::Integer(1), &Token::Float(2.5)), Token::Float(3.5));
    }

    #[test]
    fn test_add_strings() {
        assert_eq!(
            add(&Token::String("hello ".to_string()), &Token::String("world".to_string())),
            Token::String("hello world".to_string())
        );
    }

    #[test]
    fn test_add_empty_identity() {
        assert_eq!(add(&Token::Empty, &Token::Integer(5)), Token::Integer(5));
        assert_eq!(add(&Token::Integer(5), &Token::Empty), Token::Integer(5));
        assert_eq!(add(&Token::Empty, &Token::Empty), Token::Empty);
    }

    #[test]
    fn test_sub_integers() {
        assert_eq!(sub(&Token::Integer(5), &Token::Integer(3)), Token::Integer(2));
    }

    #[test]
    fn test_sub_empty_negates() {
        assert_eq!(sub(&Token::Empty, &Token::Integer(5)), Token::Integer(-5));
    }

    #[test]
    fn test_mul_integers() {
        assert_eq!(mul(&Token::Integer(3), &Token::Integer(4)), Token::Integer(12));
    }

    #[test]
    fn test_mul_empty_absorbs() {
        assert_eq!(mul(&Token::Empty, &Token::Integer(5)), Token::Integer(0));
        assert_eq!(mul(&Token::Integer(5), &Token::Empty), Token::Integer(0));
    }

    #[test]
    fn test_div_integers() {
        assert_eq!(div(&Token::Integer(10), &Token::Integer(3)), Token::Integer(3));
    }

    #[test]
    fn test_div_by_zero() {
        assert!(div(&Token::Integer(1), &Token::Integer(0)).is_error());
        assert!(div(&Token::Integer(1), &Token::Float(0.0)).is_error());
        assert!(div(&Token::Integer(1), &Token::Empty).is_error());
    }

    #[test]
    fn test_modulo() {
        assert_eq!(modulo(&Token::Integer(10), &Token::Integer(3)), Token::Integer(1));
        assert!(modulo(&Token::Integer(1), &Token::Integer(0)).is_error());
    }

    #[test]
    fn test_neg() {
        assert_eq!(neg(&Token::Integer(5)), Token::Integer(-5));
        assert_eq!(neg(&Token::Float(3.14)), Token::Float(-3.14));
        assert_eq!(neg(&Token::Empty), Token::Empty);
    }

    #[test]
    fn test_pow_integers() {
        assert_eq!(pow(&Token::Integer(2), &Token::Integer(10)), Token::Integer(1024));
        assert_eq!(pow(&Token::Integer(5), &Token::Integer(0)), Token::Integer(1));
        assert!(pow(&Token::Integer(0), &Token::Integer(0)).is_error());
    }

    #[test]
    fn test_pow_float() {
        assert_eq!(pow(&Token::Integer(2), &Token::Float(0.5)), Token::Float(2.0_f64.sqrt()));
    }

    #[test]
    fn test_comparisons() {
        assert_eq!(lt(&Token::Integer(1), &Token::Integer(2)), Token::Integer(1));
        assert_eq!(lt(&Token::Integer(2), &Token::Integer(1)), Token::Integer(0));
        assert_eq!(le(&Token::Integer(1), &Token::Integer(1)), Token::Integer(1));
        assert_eq!(gt(&Token::Integer(2), &Token::Integer(1)), Token::Integer(1));
        assert_eq!(ge(&Token::Integer(1), &Token::Integer(1)), Token::Integer(1));
        assert_eq!(eq(&Token::Integer(1), &Token::Integer(1)), Token::Integer(1));
        assert_eq!(ne(&Token::Integer(1), &Token::Integer(2)), Token::Integer(1));
    }

    #[test]
    fn test_cmp_empty_coercion() {
        assert_eq!(lt(&Token::Empty, &Token::Integer(1)), Token::Integer(1));
        assert_eq!(eq(&Token::Empty, &Token::Integer(0)), Token::Integer(1));
        assert_eq!(eq(&Token::Empty, &Token::Empty), Token::Integer(1));
    }

    #[test]
    fn test_cmp_strings() {
        assert_eq!(
            lt(&Token::String("a".to_string()), &Token::String("b".to_string())),
            Token::Integer(1)
        );
    }

    #[test]
    fn test_error_propagation() {
        let err = Token::Error("test".to_string());
        assert!(add(&err, &Token::Integer(1)).is_error());
        assert!(add(&Token::Integer(1), &err).is_error());
        assert!(sub(&err, &Token::Integer(1)).is_error());
        assert!(mul(&err, &Token::Integer(1)).is_error());
        assert!(div(&err, &Token::Integer(1)).is_error());
        assert!(lt(&err, &Token::Integer(1)).is_error());
    }

    #[test]
    fn test_about_eq() {
        assert_eq!(about_eq(&Token::Float(1.0), &Token::Float(1.0)), Token::Integer(1));
        assert_eq!(about_eq(&Token::Float(1.0), &Token::Float(2.0)), Token::Integer(0));
    }
}
