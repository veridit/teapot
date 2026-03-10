use crate::parser::EvalContext;
use crate::token::Token;
use chrono::{DateTime, Local, TimeZone};
use std::time::{SystemTime, UNIX_EPOCH};

/// Dispatch a function call by name
pub fn call_function(name: &str, args: &[Token], ctx: &mut EvalContext) -> Token {
    match name {
        // Cell reference functions
        "@" => func_at(args, ctx),
        "&" => func_adr(args, ctx),
        "x" => func_x(args, ctx),
        "y" => func_y(args, ctx),
        "z" => func_z(args, ctx),

        // Type conversion
        "int" => func_int(args),
        "float" => func_float(args),
        "frac" => func_frac(args),
        "string" => func_string(args),

        // Math
        "abs" => sci_func(args, f64::abs),
        "sin" => sci_func(args, f64::sin),
        "cos" => sci_func(args, f64::cos),
        "tan" => sci_func(args, f64::tan),
        "asin" => sci_func(args, f64::asin),
        "acos" => sci_func(args, f64::acos),
        "atan" => sci_func(args, f64::atan),
        "sinh" => sci_func(args, f64::sinh),
        "cosh" => sci_func(args, f64::cosh),
        "tanh" => sci_func(args, f64::tanh),
        "arsinh" => sci_func(args, f64::asinh),
        "arcosh" => sci_func(args, f64::acosh),
        "artanh" => sci_func(args, f64::atanh),
        "deg2rad" => sci_func(args, |x| x * std::f64::consts::PI / 180.0),
        "rad2deg" => sci_func(args, |x| x * 180.0 / std::f64::consts::PI),
        "log" => func_log(args),
        "e" => func_e(args),
        "rnd" => func_rnd(args),

        // String
        "len" => func_len(args),
        "substr" => func_substr(args),

        // Aggregates
        "sum" => func_sum(args, ctx),
        "n" => func_n(args, ctx),
        "min" => func_min(args, ctx),
        "max" => func_max(args, ctx),

        // Utility
        "eval" => func_eval(args, ctx),
        "error" => func_error(args),
        "$" => func_env(args),
        "poly" => func_poly(args),
        "time" => func_time(),
        "clock" => func_clock(args, ctx),
        "strftime" => func_strftime(args),
        "strptime" => func_strptime(args),

        _ => Token::Error(format!("unknown function: {}", name)),
    }
}

/// Helper: extract a float from a token for math functions
fn to_float(token: &Token) -> Result<f64, Token> {
    match token {
        Token::Float(f) => Ok(*f),
        Token::Integer(i) => Ok(*i as f64),
        Token::Empty => Ok(0.0),
        Token::Error(_) => Err(token.clone()),
        _ => Err(Token::Error("wrong argument type".to_string())),
    }
}

/// Helper: extract a location from a token
fn to_location(token: &Token) -> Result<(usize, usize, usize), Token> {
    match token {
        Token::Location(loc) => Ok((loc[0], loc[1], loc[2])),
        Token::Error(_) => Err(token.clone()),
        _ => Err(Token::Error("location expected".to_string())),
    }
}

/// Generic scientific function: takes one numeric argument, returns float
fn sci_func(args: &[Token], f: impl Fn(f64) -> f64) -> Token {
    if args.is_empty() {
        return Token::Error("argument expected".to_string());
    }
    match to_float(&args[0]) {
        Ok(x) => {
            let result = f(x);
            if result.is_infinite() || result.is_nan() {
                Token::Error("math domain error".to_string())
            } else {
                Token::Float(result)
            }
        }
        Err(e) => e,
    }
}

// --- Cell reference functions ---

/// Helper: resolve a coordinate arg, defaulting to the given current value if Empty
fn coord_or_default(arg: &Token, default: usize) -> Result<usize, Token> {
    match arg {
        Token::Empty => Ok(default),
        _ => to_float(arg).map(|v| v as usize),
    }
}

fn func_at(args: &[Token], ctx: &mut EvalContext) -> Token {
    // @() — value at current cell
    if args.is_empty() {
        return ctx.sheet.getvalue(ctx.x, ctx.y, ctx.z);
    }
    // @(location) or @("label")
    if args.len() == 1 {
        match &args[0] {
            Token::Location(loc) => {
                return ctx.sheet.getvalue(loc[0], loc[1], loc[2]);
            }
            Token::String(label) => {
                if let Some((x, y, z)) = ctx.sheet.findlabel_location(label) {
                    return ctx.sheet.getvalue(x, y, z);
                } else {
                    return Token::Error(format!("label '{}' not found", label));
                }
            }
            Token::Error(_) => return args[0].clone(),
            _ => {
                // @(x) — single numeric arg means (x, cur_y, cur_z)
                match coord_or_default(&args[0], ctx.x) {
                    Ok(x) => return ctx.sheet.getvalue(x, ctx.y, ctx.z),
                    Err(e) => return e,
                }
            }
        }
    }
    // @(x,y) — two args means (x, y, cur_z)
    if args.len() == 2 {
        match (coord_or_default(&args[0], ctx.x), coord_or_default(&args[1], ctx.y)) {
            (Ok(x), Ok(y)) => return ctx.sheet.getvalue(x, y, ctx.z),
            (Err(e), _) | (_, Err(e)) => return e,
        }
    }
    // @(x,y,z) — three args, Empty means use current
    match (
        coord_or_default(&args[0], ctx.x),
        coord_or_default(&args[1], ctx.y),
        coord_or_default(&args[2], ctx.z),
    ) {
        (Ok(x), Ok(y), Ok(z)) => ctx.sheet.getvalue(x, y, z),
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => e,
    }
}

fn func_adr(args: &[Token], ctx: &EvalContext) -> Token {
    // &() — location of current cell
    if args.is_empty() {
        return Token::Location([ctx.x, ctx.y, ctx.z]);
    }
    // &(x) — (x, cur_y, cur_z)
    if args.len() == 1 {
        match coord_or_default(&args[0], ctx.x) {
            Ok(x) => return Token::Location([x, ctx.y, ctx.z]),
            Err(e) => return e,
        }
    }
    // &(x,y) — (x, y, cur_z)
    if args.len() == 2 {
        match (coord_or_default(&args[0], ctx.x), coord_or_default(&args[1], ctx.y)) {
            (Ok(x), Ok(y)) => return Token::Location([x, y, ctx.z]),
            (Err(e), _) | (_, Err(e)) => return e,
        }
    }
    // &(x,y,z) — Empty means use current
    match (
        coord_or_default(&args[0], ctx.x),
        coord_or_default(&args[1], ctx.y),
        coord_or_default(&args[2], ctx.z),
    ) {
        (Ok(x), Ok(y), Ok(z)) => Token::Location([x, y, z]),
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => e,
    }
}

fn func_x(args: &[Token], ctx: &EvalContext) -> Token {
    if args.is_empty() || args[0].is_empty() {
        return Token::Integer(ctx.x as i64);
    }
    if let Ok((x, _, _)) = to_location(&args[0]) {
        Token::Integer(x as i64)
    } else {
        Token::Error("location expected".to_string())
    }
}

fn func_y(args: &[Token], ctx: &EvalContext) -> Token {
    if args.is_empty() || args[0].is_empty() {
        return Token::Integer(ctx.y as i64);
    }
    if let Ok((_, y, _)) = to_location(&args[0]) {
        Token::Integer(y as i64)
    } else {
        Token::Error("location expected".to_string())
    }
}

fn func_z(args: &[Token], ctx: &EvalContext) -> Token {
    if args.is_empty() || args[0].is_empty() {
        return Token::Integer(ctx.z as i64);
    }
    if let Ok((_, _, z)) = to_location(&args[0]) {
        Token::Integer(z as i64)
    } else {
        Token::Error("location expected".to_string())
    }
}

// --- Type conversion ---

fn func_int(args: &[Token]) -> Token {
    if args.is_empty() {
        return Token::Error("argument expected".to_string());
    }
    match &args[0] {
        Token::Integer(_) => args[0].clone(),
        Token::Float(f) => Token::Integer(*f as i64),
        Token::Empty => Token::Integer(0),
        Token::Error(_) => args[0].clone(),
        _ => Token::Error("wrong argument type for int()".to_string()),
    }
}

fn func_float(args: &[Token]) -> Token {
    if args.is_empty() {
        return Token::Error("argument expected".to_string());
    }
    match to_float(&args[0]) {
        Ok(f) => Token::Float(f),
        Err(e) => e,
    }
}

fn func_frac(args: &[Token]) -> Token {
    if args.is_empty() {
        return Token::Error("argument expected".to_string());
    }
    match &args[0] {
        Token::Float(f) => Token::Float(f.fract()),
        Token::Integer(_) => Token::Integer(0),
        Token::Empty => Token::Integer(0),
        Token::Error(_) => args[0].clone(),
        _ => Token::Error("wrong argument type for frac()".to_string()),
    }
}

fn func_string(args: &[Token]) -> Token {
    if args.is_empty() {
        return Token::Error("argument expected".to_string());
    }
    match &args[0] {
        Token::String(_) => args[0].clone(),
        Token::Integer(i) => Token::String(i.to_string()),
        Token::Float(f) => Token::String(f.to_string()),
        Token::Empty => Token::String(String::new()),
        Token::Error(_) => args[0].clone(),
        _ => Token::Error("wrong argument type for string()".to_string()),
    }
}

// --- String functions ---

fn func_len(args: &[Token]) -> Token {
    if args.is_empty() {
        return Token::Error("argument expected".to_string());
    }
    match &args[0] {
        Token::String(s) => Token::Integer(s.len() as i64),
        Token::Empty => Token::Integer(0),
        Token::Error(_) => args[0].clone(),
        _ => Token::Error("string expected for len()".to_string()),
    }
}

fn func_substr(args: &[Token]) -> Token {
    if args.len() < 3 {
        return Token::Error("substr requires 3 arguments".to_string());
    }
    match (&args[0], to_float(&args[1]), to_float(&args[2])) {
        (Token::String(s), Ok(start), Ok(len)) => {
            let start = start as usize;
            let len = len as usize;
            if start >= s.len() {
                Token::String(String::new())
            } else {
                let end = (start + len).min(s.len());
                Token::String(s[start..end].to_string())
            }
        }
        (Token::Error(_), _, _) => args[0].clone(),
        (_, Err(e), _) | (_, _, Err(e)) => e,
        _ => Token::Error("wrong argument types for substr()".to_string()),
    }
}

// --- Aggregate functions ---

/// Iterate over a range of cells defined by two location arguments
fn iterate_range(args: &[Token], ctx: &mut EvalContext, mut f: impl FnMut(Token)) -> Result<(), Token> {
    if args.len() < 2 {
        return Err(Token::Error("range requires 2 location arguments".to_string()));
    }
    let (x1, y1, z1) = to_location(&args[0])?;
    let (x2, y2, z2) = to_location(&args[1])?;
    let (xmin, xmax) = (x1.min(x2), x1.max(x2));
    let (ymin, ymax) = (y1.min(y2), y1.max(y2));
    let (zmin, zmax) = (z1.min(z2), z1.max(z2));

    for z in zmin..=zmax {
        for y in ymin..=ymax {
            for x in xmin..=xmax {
                let val = ctx.sheet.getvalue(x, y, z);
                if val.is_error() {
                    return Err(val);
                }
                f(val);
            }
        }
    }
    Ok(())
}

fn func_sum(args: &[Token], ctx: &mut EvalContext) -> Token {
    let mut total = Token::Empty;
    match iterate_range(args, ctx, |val| {
        total = crate::eval::add(&total, &val);
    }) {
        Ok(()) => total,
        Err(e) => e,
    }
}

fn func_n(args: &[Token], ctx: &mut EvalContext) -> Token {
    let mut count: i64 = 0;
    match iterate_range(args, ctx, |val| {
        if !val.is_empty() {
            count += 1;
        }
    }) {
        Ok(()) => Token::Integer(count),
        Err(e) => e,
    }
}

fn func_min(args: &[Token], ctx: &mut EvalContext) -> Token {
    let mut result = Token::Empty;
    match iterate_range(args, ctx, |val| {
        if !val.is_empty() {
            if result.is_empty() {
                result = val;
            } else if let Token::Integer(1) = crate::eval::lt(&val, &result) {
                result = val;
            }
        }
    }) {
        Ok(()) => result,
        Err(e) => e,
    }
}

fn func_max(args: &[Token], ctx: &mut EvalContext) -> Token {
    let mut result = Token::Empty;
    match iterate_range(args, ctx, |val| {
        if !val.is_empty() {
            if result.is_empty() {
                result = val;
            } else if let Token::Integer(1) = crate::eval::gt(&val, &result) {
                result = val;
            }
        }
    }) {
        Ok(()) => result,
        Err(e) => e,
    }
}

// --- Math utility ---

fn func_e(args: &[Token]) -> Token {
    if args.is_empty() || args[0].is_empty() {
        return Token::Float(std::f64::consts::E);
    }
    match to_float(&args[0]) {
        Ok(x) => {
            let result = x.exp();
            if result.is_infinite() || result.is_nan() {
                Token::Error("math domain error".to_string())
            } else {
                Token::Float(result)
            }
        }
        Err(e) => e,
    }
}

fn func_log(args: &[Token]) -> Token {
    if args.is_empty() {
        return Token::Error("argument expected".to_string());
    }
    match to_float(&args[0]) {
        Ok(x) => {
            let result = if args.len() >= 2 {
                // log(x, base)
                match to_float(&args[1]) {
                    Ok(base) => x.ln() / base.ln(),
                    Err(e) => return e,
                }
            } else {
                // log(x) — natural log
                x.ln()
            };
            if result.is_infinite() || result.is_nan() {
                Token::Error("math domain error".to_string())
            } else {
                Token::Float(result)
            }
        }
        Err(e) => e,
    }
}

fn func_rnd(_args: &[Token]) -> Token {
    // rnd() — returns uniform random float in [0, 1)
    // Use system time to seed a simple random value
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // xorshift-style mixing of the nanosecond timestamp
    let mut s = t as u64;
    s ^= s >> 12;
    s ^= s << 25;
    s ^= s >> 27;
    s = s.wrapping_mul(0x2545F4914F6CDD1D);
    Token::Float((s >> 11) as f64 / (1u64 << 53) as f64)
}

fn func_poly(args: &[Token]) -> Token {
    if args.len() < 2 {
        return Token::Error("poly requires at least 2 arguments".to_string());
    }
    match to_float(&args[0]) {
        Ok(x) => {
            let mut result = 0.0;
            for (i, arg) in args[1..].iter().enumerate() {
                match to_float(arg) {
                    Ok(coeff) => result += coeff * x.powi(i as i32),
                    Err(e) => return e,
                }
            }
            Token::Float(result)
        }
        Err(e) => e,
    }
}

// --- Utility functions ---

fn func_eval(args: &[Token], ctx: &mut EvalContext) -> Token {
    if args.is_empty() {
        return Token::Error("argument expected".to_string());
    }
    let (x, y, z) = match &args[0] {
        Token::Location(loc) => (loc[0], loc[1], loc[2]),
        _ => return args[0].clone(),
    };

    // Prevent infinite recursion
    if ctx.max_eval <= 0 {
        return Token::Error("max eval depth exceeded".to_string());
    }
    ctx.max_eval -= 1;

    // Take the cell's contents out (same take/put pattern as Sheet::eval_cell)
    let contents = ctx.sheet.cells_mut()
        .get_mut(&(x, y, z))
        .and_then(|c| c.contents.take());

    let result = if let Some(tokens) = &contents {
        // Save and set context coordinates
        let (old_x, old_y, old_z) = (ctx.x, ctx.y, ctx.z);
        ctx.x = x;
        ctx.y = y;
        ctx.z = z;
        let result = crate::parser::eval_tokens(tokens, ctx);
        ctx.x = old_x;
        ctx.y = old_y;
        ctx.z = old_z;
        result
    } else {
        Token::Empty
    };

    // Put contents back
    if let Some(tokens) = contents {
        if let Some(cell) = ctx.sheet.cells_mut().get_mut(&(x, y, z)) {
            cell.contents = Some(tokens);
        }
    }

    ctx.max_eval += 1;
    result
}

fn func_error(args: &[Token]) -> Token {
    if args.is_empty() {
        return Token::Error("error".to_string());
    }
    match &args[0] {
        Token::String(s) => Token::Error(s.clone()),
        Token::Error(_) => args[0].clone(),
        _ => Token::Error("error".to_string()),
    }
}

fn func_env(args: &[Token]) -> Token {
    if args.is_empty() {
        return Token::Error("argument expected".to_string());
    }
    match &args[0] {
        Token::String(name) => {
            match std::env::var(name) {
                Ok(val) => Token::String(val),
                Err(_) => Token::String(String::new()),
            }
        }
        Token::Error(_) => args[0].clone(),
        _ => Token::Error("string expected for $()".to_string()),
    }
}

fn func_time() -> Token {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => Token::Integer(d.as_secs() as i64),
        Err(_) => Token::Error("time error".to_string()),
    }
}

fn func_clock(args: &[Token], ctx: &EvalContext) -> Token {
    // Return the clocked value of the current cell
    if let Some(cell) = ctx.sheet.get_cell(ctx.x, ctx.y, ctx.z) {
        if !cell.clocked_value.is_empty() {
            return cell.clocked_value.clone();
        }
    }
    if args.is_empty() || args[0].is_empty() {
        Token::Empty
    } else {
        args[0].clone()
    }
}

fn func_strftime(args: &[Token]) -> Token {
    if args.is_empty() {
        return Token::Error("strftime requires format string".to_string());
    }
    let fmt = match &args[0] {
        Token::String(s) => s.as_str(),
        Token::Error(_) => return args[0].clone(),
        _ => return Token::Error("string expected for strftime format".to_string()),
    };
    let dt: DateTime<Local> = if args.len() >= 2 {
        match to_float(&args[1]) {
            Ok(ts) => match Local.timestamp_opt(ts as i64, 0) {
                chrono::LocalResult::Single(dt) => dt,
                _ => return Token::Error("invalid timestamp".to_string()),
            },
            Err(e) => return e,
        }
    } else {
        Local::now()
    };
    Token::String(dt.format(fmt).to_string())
}

fn func_strptime(args: &[Token]) -> Token {
    // Simplified: parse a time string
    if args.len() < 2 {
        return Token::Error("strptime requires format and string".to_string());
    }
    // Would need chrono for full implementation
    Token::Error("strptime not yet implemented".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sheet::Sheet;

    fn make_ctx(sheet: &mut Sheet) -> EvalContext<'_> {
        EvalContext {
            sheet,
            x: 0, y: 0, z: 0,
            max_eval: 256,
        }
    }

    #[test]
    fn test_abs() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("abs", &[Token::Float(-3.14)], &mut ctx), Token::Float(3.14));
        assert_eq!(call_function("abs", &[Token::Integer(-5)], &mut ctx), Token::Float(5.0));
    }

    #[test]
    fn test_int_conversion() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("int", &[Token::Float(3.7)], &mut ctx), Token::Integer(3));
        assert_eq!(call_function("int", &[Token::Integer(5)], &mut ctx), Token::Integer(5));
    }

    #[test]
    fn test_float_conversion() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("float", &[Token::Integer(5)], &mut ctx), Token::Float(5.0));
    }

    #[test]
    fn test_string_conversion() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("string", &[Token::Integer(42)], &mut ctx), Token::String("42".to_string()));
    }

    #[test]
    fn test_len() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("len", &[Token::String("hello".to_string())], &mut ctx), Token::Integer(5));
    }

    #[test]
    fn test_substr() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(
            call_function("substr", &[Token::String("hello".to_string()), Token::Integer(1), Token::Integer(3)], &mut ctx),
            Token::String("ell".to_string())
        );
    }

    #[test]
    fn test_adr() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(
            call_function("&", &[Token::Integer(1), Token::Integer(2), Token::Integer(0)], &mut ctx),
            Token::Location([1, 2, 0])
        );
    }

    #[test]
    fn test_xyz() {
        let mut sheet = Sheet::new();
        let mut ctx = EvalContext { sheet: &mut sheet, x: 5, y: 10, z: 2, max_eval: 256 };
        assert_eq!(call_function("x", &[Token::Empty], &mut ctx), Token::Integer(5));
        assert_eq!(call_function("y", &[Token::Empty], &mut ctx), Token::Integer(10));
        assert_eq!(call_function("z", &[Token::Empty], &mut ctx), Token::Integer(2));
    }

    #[test]
    fn test_e_constant() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("e", &[Token::Empty], &mut ctx), Token::Float(std::f64::consts::E));
    }

    #[test]
    fn test_sin() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("sin", &[Token::Float(0.0)], &mut ctx), Token::Float(0.0));
    }

    #[test]
    fn test_error_func() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("error", &[Token::String("test error".to_string())], &mut ctx);
        assert!(result.is_error());
        assert_eq!(result.error_message(), Some("test error"));
    }

    #[test]
    fn test_time() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("time", &[], &mut ctx);
        match result {
            Token::Integer(t) => assert!(t > 0),
            _ => panic!("expected integer timestamp"),
        }
    }
}
