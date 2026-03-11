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
        "string" => func_string(args, ctx),

        // Math
        "abs" => func_abs(args),
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
    // 1-arg form: simple conversion
    if args.len() == 1 {
        return match &args[0] {
            Token::Integer(_) => args[0].clone(),
            Token::Float(f) => Token::Integer(*f as i64),
            Token::Empty => Token::Integer(0),
            Token::String(s) => {
                // Parse string as integer (like C strtol)
                let s = s.trim();
                if let Ok(i) = s.parse::<i64>() {
                    Token::Integer(i)
                } else if let Ok(f) = s.parse::<f64>() {
                    Token::Integer(f as i64)
                } else {
                    Token::Error(format!("cannot convert '{}' to int", s))
                }
            }
            Token::Error(_) => args[0].clone(),
            _ => Token::Error("wrong argument type for int()".to_string()),
        };
    }
    // 3-arg form: int(float, neg_mode, pos_mode)
    if args.len() >= 3 {
        let f = match to_float(&args[0]) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let neg_mode = match to_float(&args[1]) {
            Ok(v) => v as i64,
            Err(e) => return e,
        };
        let pos_mode = match to_float(&args[2]) {
            Ok(v) => v as i64,
            Err(e) => return e,
        };
        let mode = if f < 0.0 { neg_mode } else { pos_mode };
        let result = match mode {
            m if m < -1 => f.floor() as i64,   // floor
            -1 => {                              // round away from zero
                if f < 0.0 { f.floor() as i64 } else { f.ceil() as i64 }
            }
            0 => f as i64,                       // truncate (toward zero)
            1 => {                               // round toward zero
                if f < 0.0 { f.ceil() as i64 } else { f.floor() as i64 }
            }
            _ => f.ceil() as i64,                // ceil (m > 1)
        };
        return Token::Integer(result);
    }
    // 2-arg form: treat as 1-arg (ignore extra)
    match to_float(&args[0]) {
        Ok(f) => Token::Integer(f as i64),
        Err(e) => e,
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

fn func_string(args: &[Token], ctx: &mut EvalContext) -> Token {
    if args.is_empty() {
        return Token::Error("argument expected".to_string());
    }
    // string(location) — format cell value using cell's own precision/scientific settings
    if args.len() == 1 {
        if let Token::Location(loc) = &args[0] {
            let (x, y, z) = (loc[0], loc[1], loc[2]);
            let value = ctx.sheet.getvalue(x, y, z);
            let (precision, scientific) = ctx.sheet.get_cell(x, y, z)
                .map(|c| (c.precision, c.scientific))
                .unwrap_or((-1, false));
            return match value {
                Token::Float(f) => {
                    if precision >= 0 {
                        if scientific {
                            Token::String(format!("{:.prec$e}", f, prec = precision as usize))
                        } else {
                            Token::String(format!("{:.prec$}", f, prec = precision as usize))
                        }
                    } else {
                        Token::String(f.to_string())
                    }
                }
                Token::Integer(i) => Token::String(i.to_string()),
                Token::String(_) => value,
                Token::Empty => Token::String(String::new()),
                Token::Error(_) => value,
                _ => Token::Error("wrong argument type for string()".to_string()),
            };
        }
    }
    // With precision (and optional mode) arguments
    if args.len() >= 2 {
        let val = match to_float(&args[0]) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let precision = match to_float(&args[1]) {
            Ok(v) => v as usize,
            Err(e) => return e,
        };
        let scientific = if args.len() >= 3 {
            match to_float(&args[2]) {
                Ok(v) => v as i64 != 0,
                Err(e) => return e,
            }
        } else {
            false
        };
        return if scientific {
            Token::String(format!("{:.prec$e}", val, prec = precision))
        } else {
            Token::String(format!("{:.prec$}", val, prec = precision))
        };
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
        (Token::String(s), Ok(start), Ok(end)) => {
            let start = start as usize;
            let end = end as usize;
            if start >= s.len() {
                Token::String(String::new())
            } else {
                let actual_end = (end + 1).min(s.len());
                if start > end {
                    Token::String(String::new())
                } else {
                    Token::String(s[start..actual_end].to_string())
                }
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
    if args.len() < 2 {
        return Token::Error("range requires 2 location arguments".to_string());
    }
    let (x1, y1, z1) = match to_location(&args[0]) { Ok(v) => v, Err(e) => return e };
    let (x2, y2, z2) = match to_location(&args[1]) { Ok(v) => v, Err(e) => return e };
    let (xmin, xmax) = (x1.min(x2), x1.max(x2));
    let (ymin, ymax) = (y1.min(y2), y1.max(y2));
    let (zmin, zmax) = (z1.min(z2), z1.max(z2));

    let mut best_val = Token::Empty;
    let mut best_loc: (usize, usize, usize) = (xmin, ymin, zmin);

    for z in zmin..=zmax {
        for y in ymin..=ymax {
            for x in xmin..=xmax {
                let val = ctx.sheet.getvalue(x, y, z);
                if val.is_error() {
                    return val;
                }
                if !val.is_empty() {
                    if best_val.is_empty() {
                        best_val = val;
                        best_loc = (x, y, z);
                    } else if let Token::Integer(1) = crate::eval::lt(&val, &best_val) {
                        best_val = val;
                        best_loc = (x, y, z);
                    }
                }
            }
        }
    }

    if best_val.is_empty() {
        Token::Empty
    } else {
        Token::Location([best_loc.0, best_loc.1, best_loc.2])
    }
}

fn func_max(args: &[Token], ctx: &mut EvalContext) -> Token {
    if args.len() < 2 {
        return Token::Error("range requires 2 location arguments".to_string());
    }
    let (x1, y1, z1) = match to_location(&args[0]) { Ok(v) => v, Err(e) => return e };
    let (x2, y2, z2) = match to_location(&args[1]) { Ok(v) => v, Err(e) => return e };
    let (xmin, xmax) = (x1.min(x2), x1.max(x2));
    let (ymin, ymax) = (y1.min(y2), y1.max(y2));
    let (zmin, zmax) = (z1.min(z2), z1.max(z2));

    let mut best_val = Token::Empty;
    let mut best_loc: (usize, usize, usize) = (xmin, ymin, zmin);

    for z in zmin..=zmax {
        for y in ymin..=ymax {
            for x in xmin..=xmax {
                let val = ctx.sheet.getvalue(x, y, z);
                if val.is_error() {
                    return val;
                }
                if !val.is_empty() {
                    if best_val.is_empty() {
                        best_val = val;
                        best_loc = (x, y, z);
                    } else if let Token::Integer(1) = crate::eval::gt(&val, &best_val) {
                        best_val = val;
                        best_loc = (x, y, z);
                    }
                }
            }
        }
    }

    if best_val.is_empty() {
        Token::Empty
    } else {
        Token::Location([best_loc.0, best_loc.1, best_loc.2])
    }
}

/// abs() — preserve integer type for integer input
fn func_abs(args: &[Token]) -> Token {
    if args.is_empty() {
        return Token::Error("argument expected".to_string());
    }
    match &args[0] {
        Token::Integer(i) => Token::Integer(i.abs()),
        Token::Float(f) => Token::Float(f.abs()),
        Token::Empty => Token::Integer(0),
        Token::Error(_) => args[0].clone(),
        _ => Token::Error("wrong argument type".to_string()),
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

fn func_clock(args: &[Token], ctx: &mut EvalContext) -> Token {
    // clock(condition, location[, location]) — enable clock_t2 on cells
    if args.len() >= 2 {
        if let Ok(condition) = to_float(&args[0]) {
            if condition as i64 != 0 {
                // Enable clock on the specified location(s)
                if let Ok((x, y, z)) = to_location(&args[1]) {
                    enable_clock_on_cell(ctx, x, y, z);
                }
                if args.len() >= 3 {
                    if let Ok((x, y, z)) = to_location(&args[2]) {
                        enable_clock_on_cell(ctx, x, y, z);
                    }
                }
            }
            return Token::Integer(condition as i64);
        }
    }
    // 0-arg / 1-arg form: return the clocked value of the current cell
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

/// Enable clock_t2 on a cell (used by clock(condition, location) form)
fn enable_clock_on_cell(ctx: &mut EvalContext, x: usize, y: usize, z: usize) {
    let cell = ctx.sheet.get_or_create_cell(x, y, z);
    if !cell.clock_t2 {
        cell.clock_t2 = true;
        cell.clocked_contents = cell.contents.clone();
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
    if args.len() < 2 {
        return Token::Error("strptime requires format and string".to_string());
    }
    let fmt = match &args[0] {
        Token::String(s) => s.as_str(),
        Token::Error(_) => return args[0].clone(),
        _ => return Token::Error("string expected for strptime format".to_string()),
    };
    let input = match &args[1] {
        Token::String(s) => s.as_str(),
        Token::Error(_) => return args[1].clone(),
        _ => return Token::Error("string expected for strptime input".to_string()),
    };
    // Try parsing as NaiveDateTime first, then NaiveDate
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(input, fmt) {
        return Token::Integer(dt.and_utc().timestamp());
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(input, fmt) {
        let dt = d.and_hms_opt(0, 0, 0).unwrap();
        return Token::Integer(dt.and_utc().timestamp());
    }
    Token::Error(format!("strptime: cannot parse '{}' with format '{}'", input, fmt))
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
        assert_eq!(call_function("abs", &[Token::Integer(-5)], &mut ctx), Token::Integer(5));
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

    // --- Cell reference tests ---

    #[test]
    fn test_at_no_args() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(42)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("@", &[], &mut ctx), Token::Integer(42));
    }

    #[test]
    fn test_at_single_coord() {
        let mut sheet = Sheet::new();
        sheet.putcont(3, 0, 0, vec![Token::Integer(99)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("@", &[Token::Integer(3)], &mut ctx), Token::Integer(99));
    }

    #[test]
    fn test_at_two_coords() {
        let mut sheet = Sheet::new();
        sheet.putcont(2, 3, 0, vec![Token::Integer(77)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("@", &[Token::Integer(2), Token::Integer(3)], &mut ctx), Token::Integer(77));
    }

    #[test]
    fn test_at_three_coords() {
        let mut sheet = Sheet::new();
        sheet.putcont(1, 2, 1, vec![Token::Integer(55)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("@", &[Token::Integer(1), Token::Integer(2), Token::Integer(1)], &mut ctx), Token::Integer(55));
    }

    #[test]
    fn test_at_location() {
        let mut sheet = Sheet::new();
        sheet.putcont(1, 1, 0, vec![Token::Integer(88)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("@", &[Token::Location([1, 1, 0])], &mut ctx), Token::Integer(88));
    }

    #[test]
    fn test_at_label() {
        let mut sheet = Sheet::new();
        sheet.putcont(2, 0, 0, vec![Token::Integer(100)]);
        sheet.get_or_create_cell(2, 0, 0).label = Some("myval".to_string());
        sheet.cachelabels();
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("@", &[Token::String("myval".to_string())], &mut ctx), Token::Integer(100));
    }

    #[test]
    fn test_at_empty_coords() {
        // @(,y,) uses defaults for x and z
        let mut sheet = Sheet::new();
        sheet.putcont(0, 5, 0, vec![Token::Integer(33)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("@", &[Token::Empty, Token::Integer(5), Token::Empty], &mut ctx), Token::Integer(33));
    }

    #[test]
    fn test_adr_no_args() {
        let mut sheet = Sheet::new();
        let mut ctx = EvalContext { sheet: &mut sheet, x: 3, y: 7, z: 1, max_eval: 256 };
        assert_eq!(call_function("&", &[], &mut ctx), Token::Location([3, 7, 1]));
    }

    #[test]
    fn test_adr_partial() {
        let mut sheet = Sheet::new();
        let mut ctx = EvalContext { sheet: &mut sheet, x: 3, y: 7, z: 1, max_eval: 256 };
        assert_eq!(call_function("&", &[Token::Integer(5)], &mut ctx), Token::Location([5, 7, 1]));
        assert_eq!(call_function("&", &[Token::Integer(5), Token::Integer(2)], &mut ctx), Token::Location([5, 2, 1]));
    }

    // --- Aggregate tests ---

    #[test]
    fn test_sum_range() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(10)]);
        sheet.putcont(0, 1, 0, vec![Token::Integer(20)]);
        sheet.putcont(0, 2, 0, vec![Token::Integer(30)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(
            call_function("sum", &[Token::Location([0, 0, 0]), Token::Location([0, 2, 0])], &mut ctx),
            Token::Integer(60)
        );
    }

    #[test]
    fn test_sum_empty_cells() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(10)]);
        // cell (0,1,0) is empty
        sheet.putcont(0, 2, 0, vec![Token::Integer(30)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(
            call_function("sum", &[Token::Location([0, 0, 0]), Token::Location([0, 2, 0])], &mut ctx),
            Token::Integer(40)
        );
    }

    #[test]
    fn test_n_count() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(10)]);
        // cell (0,1,0) is empty
        sheet.putcont(0, 2, 0, vec![Token::Integer(30)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(
            call_function("n", &[Token::Location([0, 0, 0]), Token::Location([0, 2, 0])], &mut ctx),
            Token::Integer(2)
        );
    }

    #[test]
    fn test_min_returns_location() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(42)]);
        sheet.putcont(0, 1, 0, vec![Token::Integer(99)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("min", &[Token::Location([0, 0, 0]), Token::Location([0, 1, 0])], &mut ctx);
        assert_eq!(result, Token::Location([0, 0, 0])); // min is 42 at (0,0,0)
    }

    #[test]
    fn test_max_returns_location() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(42)]);
        sheet.putcont(0, 1, 0, vec![Token::Integer(99)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("max", &[Token::Location([0, 0, 0]), Token::Location([0, 1, 0])], &mut ctx);
        assert_eq!(result, Token::Location([0, 1, 0])); // max is 99 at (0,1,0)
    }

    // --- Math tests ---

    #[test]
    fn test_log_natural() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("log", &[Token::Float(std::f64::consts::E)], &mut ctx);
        match result {
            Token::Float(f) => assert!((f - 1.0).abs() < 1e-10),
            _ => panic!("expected float"),
        }
    }

    #[test]
    fn test_log_base10() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("log", &[Token::Float(100.0), Token::Integer(10)], &mut ctx);
        match result {
            Token::Float(f) => assert!((f - 2.0).abs() < 1e-10),
            _ => panic!("expected float"),
        }
    }

    #[test]
    fn test_log_domain_error() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert!(call_function("log", &[Token::Float(-1.0)], &mut ctx).is_error());
    }

    #[test]
    fn test_abs_preserves_int() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("abs", &[Token::Integer(-5)], &mut ctx), Token::Integer(5));
    }

    #[test]
    fn test_abs_float() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("abs", &[Token::Float(-3.14)], &mut ctx), Token::Float(3.14));
    }

    #[test]
    fn test_rnd_range() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("rnd", &[], &mut ctx);
        match result {
            Token::Float(f) => assert!((0.0..1.0).contains(&f)),
            _ => panic!("expected float"),
        }
    }

    #[test]
    fn test_frac() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("frac", &[Token::Float(3.14)], &mut ctx);
        match result {
            Token::Float(f) => assert!((f - 0.14).abs() < 1e-10),
            _ => panic!("expected float"),
        }
    }

    // --- Type conversion tests ---

    #[test]
    fn test_int_from_string() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("int", &[Token::String("42".to_string())], &mut ctx), Token::Integer(42));
    }

    #[test]
    fn test_int_rounding_modes() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        // int(2.7, neg_mode, pos_mode) — positive value, so pos_mode applies
        // mode < -1 = floor
        assert_eq!(
            call_function("int", &[Token::Float(2.7), Token::Integer(0), Token::Integer(-2)], &mut ctx),
            Token::Integer(2) // floor(2.7) = 2
        );
        // mode > 1 = ceil
        assert_eq!(
            call_function("int", &[Token::Float(2.7), Token::Integer(0), Token::Integer(2)], &mut ctx),
            Token::Integer(3) // ceil(2.7) = 3
        );
        // mode = 0 = truncate
        assert_eq!(
            call_function("int", &[Token::Float(-2.7), Token::Integer(0), Token::Integer(0)], &mut ctx),
            Token::Integer(-2) // truncate(-2.7) = -2
        );
        // mode = -1 = round away from zero (negative value, so neg_mode applies)
        assert_eq!(
            call_function("int", &[Token::Float(-2.3), Token::Integer(-1), Token::Integer(0)], &mut ctx),
            Token::Integer(-3) // floor(-2.3) = -3 (away from zero)
        );
    }

    #[test]
    fn test_int_no_args_error() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert!(call_function("int", &[], &mut ctx).is_error());
    }

    #[test]
    fn test_string_with_precision() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(
            call_function("string", &[Token::Float(3.14159), Token::Integer(2)], &mut ctx),
            Token::String("3.14".to_string())
        );
    }

    #[test]
    fn test_string_scientific() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("string", &[Token::Float(12345.0), Token::Integer(2), Token::Integer(1)], &mut ctx);
        match result {
            Token::String(s) => assert!(s.contains('e'), "expected scientific notation, got: {}", s),
            _ => panic!("expected string"),
        }
    }

    // --- String tests ---

    #[test]
    fn test_substr_inclusive() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        // substr("hello", 1, 3) = chars at indices 1,2,3 = "ell"
        assert_eq!(
            call_function("substr", &[Token::String("hello".to_string()), Token::Integer(1), Token::Integer(3)], &mut ctx),
            Token::String("ell".to_string())
        );
    }

    #[test]
    fn test_substr_boundary() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        // end beyond string length — should clamp
        assert_eq!(
            call_function("substr", &[Token::String("hi".to_string()), Token::Integer(0), Token::Integer(10)], &mut ctx),
            Token::String("hi".to_string())
        );
    }

    #[test]
    fn test_len_empty() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("len", &[Token::String(String::new())], &mut ctx), Token::Integer(0));
    }

    // --- Utility tests ---

    #[test]
    fn test_poly() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        // poly(2, 1, 3, 5) = 1 + 3*2 + 5*4 = 27
        let result = call_function("poly", &[Token::Integer(2), Token::Integer(1), Token::Integer(3), Token::Integer(5)], &mut ctx);
        assert_eq!(result, Token::Float(27.0));
    }

    #[test]
    fn test_env_existing() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("$", &[Token::String("HOME".to_string())], &mut ctx);
        match result {
            Token::String(s) => assert!(!s.is_empty()),
            _ => panic!("expected string"),
        }
    }

    #[test]
    fn test_strftime_format() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("strftime", &[Token::String("%Y".to_string())], &mut ctx);
        match result {
            Token::String(s) => assert_eq!(s.len(), 4, "expected 4-digit year, got: {}", s),
            _ => panic!("expected string"),
        }
    }

    #[test]
    fn test_strptime() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("strptime", &[
            Token::String("%Y-%m-%d".to_string()),
            Token::String("2024-01-15".to_string()),
        ], &mut ctx);
        match result {
            Token::Integer(ts) => assert!(ts > 0),
            other => panic!("expected integer timestamp, got: {:?}", other),
        }
    }

    #[test]
    fn test_clock_enable() {
        let mut sheet = Sheet::new();
        sheet.putcont(1, 1, 0, vec![Token::Integer(42)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("clock", &[Token::Integer(1), Token::Location([1, 1, 0])], &mut ctx);
        assert_eq!(result, Token::Integer(1));
        // Verify clock_t2 was set
        assert!(ctx.sheet.get_cell(1, 1, 0).unwrap().clock_t2);
    }

    #[test]
    fn test_eval_cell() {
        let mut sheet = Sheet::new();
        // Put a simple expression "5" in cell (1,0,0)
        sheet.putcont(1, 0, 0, vec![Token::Integer(5)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("eval", &[Token::Location([1, 0, 0])], &mut ctx);
        assert_eq!(result, Token::Integer(5));
    }

    // === B1: Trigonometric & Hyperbolic Functions ===

    #[test]
    fn test_cos_zero() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("cos", &[Token::Float(0.0)], &mut ctx), Token::Float(1.0));
    }

    #[test]
    fn test_tan_zero() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("tan", &[Token::Float(0.0)], &mut ctx), Token::Float(0.0));
    }

    #[test]
    fn test_asin_valid() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("asin", &[Token::Float(1.0)], &mut ctx);
        match result {
            Token::Float(f) => assert!((f - std::f64::consts::FRAC_PI_2).abs() < 1e-10),
            _ => panic!("expected float"),
        }
    }

    #[test]
    fn test_asin_domain_error() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert!(call_function("asin", &[Token::Float(2.0)], &mut ctx).is_error());
    }

    #[test]
    fn test_acos_valid() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("acos", &[Token::Float(1.0)], &mut ctx), Token::Float(0.0));
    }

    #[test]
    fn test_acos_domain_error() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert!(call_function("acos", &[Token::Float(2.0)], &mut ctx).is_error());
    }

    #[test]
    fn test_atan_zero() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("atan", &[Token::Float(0.0)], &mut ctx), Token::Float(0.0));
    }

    #[test]
    fn test_sinh_zero() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("sinh", &[Token::Float(0.0)], &mut ctx), Token::Float(0.0));
    }

    #[test]
    fn test_cosh_zero() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("cosh", &[Token::Float(0.0)], &mut ctx), Token::Float(1.0));
    }

    #[test]
    fn test_tanh_zero() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("tanh", &[Token::Float(0.0)], &mut ctx), Token::Float(0.0));
    }

    #[test]
    fn test_arsinh_zero() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("arsinh", &[Token::Float(0.0)], &mut ctx), Token::Float(0.0));
    }

    #[test]
    fn test_arcosh_one() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("arcosh", &[Token::Float(1.0)], &mut ctx), Token::Float(0.0));
    }

    #[test]
    fn test_arcosh_domain() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert!(call_function("arcosh", &[Token::Float(0.0)], &mut ctx).is_error());
    }

    #[test]
    fn test_artanh_zero() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("artanh", &[Token::Float(0.0)], &mut ctx), Token::Float(0.0));
    }

    #[test]
    fn test_artanh_domain() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        // artanh(1) = infinity → error
        assert!(call_function("artanh", &[Token::Float(1.0)], &mut ctx).is_error());
    }

    #[test]
    fn test_deg2rad_90() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("deg2rad", &[Token::Float(90.0)], &mut ctx);
        match result {
            Token::Float(f) => assert!((f - std::f64::consts::FRAC_PI_2).abs() < 1e-10),
            _ => panic!("expected float"),
        }
    }

    #[test]
    fn test_rad2deg_pi() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("rad2deg", &[Token::Float(std::f64::consts::PI)], &mut ctx);
        match result {
            Token::Float(f) => assert!((f - 180.0).abs() < 1e-10),
            _ => panic!("expected float"),
        }
    }

    #[test]
    fn test_trig_inverse_pair() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        // sin(asin(0.5)) ≈ 0.5
        let asin_result = call_function("asin", &[Token::Float(0.5)], &mut ctx);
        let result = call_function("sin", &[asin_result], &mut ctx);
        match result {
            Token::Float(f) => assert!((f - 0.5).abs() < 1e-10),
            _ => panic!("expected float"),
        }
    }

    #[test]
    fn test_trig_no_args() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert!(call_function("sin", &[], &mut ctx).is_error());
    }

    #[test]
    fn test_trig_string_arg() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert!(call_function("sin", &[Token::String("hello".to_string())], &mut ctx).is_error());
    }

    // === B2: Math Edge Cases ===

    #[test]
    fn test_e_exponentiation() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("e", &[Token::Integer(2)], &mut ctx);
        match result {
            Token::Float(f) => assert!((f - std::f64::consts::E.powi(2)).abs() < 1e-10),
            _ => panic!("expected float"),
        }
    }

    #[test]
    fn test_e_overflow() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert!(call_function("e", &[Token::Integer(1000)], &mut ctx).is_error());
    }

    #[test]
    fn test_abs_empty() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("abs", &[Token::Empty], &mut ctx), Token::Integer(0));
    }

    #[test]
    fn test_abs_error_prop() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let err = Token::Error("test".to_string());
        assert!(call_function("abs", &[err], &mut ctx).is_error());
    }

    #[test]
    fn test_log_base_one() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        // log(10, 1) → ln(10)/ln(1) = ln(10)/0 = inf → error
        assert!(call_function("log", &[Token::Float(10.0), Token::Integer(1)], &mut ctx).is_error());
    }

    #[test]
    fn test_rnd_multiple() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        // Two calls — both should be valid floats in [0,1)
        let r1 = call_function("rnd", &[], &mut ctx);
        let r2 = call_function("rnd", &[], &mut ctx);
        match (&r1, &r2) {
            (Token::Float(f1), Token::Float(f2)) => {
                assert!((0.0..1.0).contains(f1));
                assert!((0.0..1.0).contains(f2));
            }
            _ => panic!("expected floats"),
        }
    }

    #[test]
    fn test_poly_single_coeff() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        // poly(x, c0) = c0
        assert_eq!(
            call_function("poly", &[Token::Integer(5), Token::Integer(7)], &mut ctx),
            Token::Float(7.0)
        );
    }

    // === B3: Aggregate Edge Cases ===

    #[test]
    fn test_min_empty_range() {
        let mut sheet = Sheet::new();
        // All cells in range are empty
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("min", &[Token::Location([0, 0, 0]), Token::Location([0, 2, 0])], &mut ctx);
        assert_eq!(result, Token::Empty);
    }

    #[test]
    fn test_max_empty_range() {
        let mut sheet = Sheet::new();
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("max", &[Token::Location([0, 0, 0]), Token::Location([0, 2, 0])], &mut ctx);
        assert_eq!(result, Token::Empty);
    }

    #[test]
    fn test_min_single_cell() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(42)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("min", &[Token::Location([0, 0, 0]), Token::Location([0, 0, 0])], &mut ctx);
        assert_eq!(result, Token::Location([0, 0, 0]));
    }

    #[test]
    fn test_sum_mixed_types() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(10)]);
        sheet.putcont(0, 1, 0, vec![Token::Float(5.5)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("sum", &[Token::Location([0, 0, 0]), Token::Location([0, 1, 0])], &mut ctx);
        match result {
            Token::Float(f) => assert!((f - 15.5).abs() < 1e-10),
            _ => panic!("expected float, got {:?}", result),
        }
    }

    #[test]
    fn test_n_all_empty() {
        let mut sheet = Sheet::new();
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(
            call_function("n", &[Token::Location([0, 0, 0]), Token::Location([0, 2, 0])], &mut ctx),
            Token::Integer(0)
        );
    }

    #[test]
    fn test_sum_3d_range() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(10)]);
        sheet.putcont(0, 0, 1, vec![Token::Integer(20)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("sum", &[Token::Location([0, 0, 0]), Token::Location([0, 0, 1])], &mut ctx);
        assert_eq!(result, Token::Integer(30));
    }

    #[test]
    fn test_aggregate_error() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(10)]);
        // Put an error-producing expression
        sheet.putcont(0, 1, 0, vec![Token::Error("bad".to_string())]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("sum", &[Token::Location([0, 0, 0]), Token::Location([0, 1, 0])], &mut ctx);
        assert!(result.is_error());
    }

    #[test]
    fn test_min_max_at_compose() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(100)]);
        sheet.putcont(0, 1, 0, vec![Token::Integer(5)]);
        sheet.putcont(0, 2, 0, vec![Token::Integer(50)]);
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        // min returns location of min cell
        let min_loc = call_function("min", &[Token::Location([0, 0, 0]), Token::Location([0, 2, 0])], &mut ctx);
        assert_eq!(min_loc, Token::Location([0, 1, 0]));
        // @(min_loc) gets the value
        let val = call_function("@", &[min_loc], &mut ctx);
        assert_eq!(val, Token::Integer(5));
    }

    // === B4: Type Conversion Edge Cases ===

    #[test]
    fn test_int_from_float_string() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(
            call_function("int", &[Token::String("3.14".to_string())], &mut ctx),
            Token::Integer(3)
        );
    }

    #[test]
    fn test_int_invalid_string() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert!(call_function("int", &[Token::String("abc".to_string())], &mut ctx).is_error());
    }

    #[test]
    fn test_frac_negative() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("frac", &[Token::Float(-3.14)], &mut ctx);
        match result {
            Token::Float(f) => assert!((f - (-0.14)).abs() < 1e-10),
            _ => panic!("expected float"),
        }
    }

    #[test]
    fn test_frac_integer() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        // frac of integer returns Integer(0)
        assert_eq!(call_function("frac", &[Token::Integer(5)], &mut ctx), Token::Integer(0));
    }

    #[test]
    fn test_string_location() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Float(3.14159)]);
        {
            let cell = sheet.get_or_create_cell(0, 0, 0);
            cell.precision = 2;
        }
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("string", &[Token::Location([0, 0, 0])], &mut ctx);
        assert_eq!(result, Token::String("3.14".to_string()));
    }

    // === B5: String Edge Cases ===

    #[test]
    fn test_substr_start_equals_end() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(
            call_function("substr", &[Token::String("hello".to_string()), Token::Integer(2), Token::Integer(2)], &mut ctx),
            Token::String("l".to_string())
        );
    }

    #[test]
    fn test_substr_start_past_end() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(
            call_function("substr", &[Token::String("hello".to_string()), Token::Integer(3), Token::Integer(1)], &mut ctx),
            Token::String(String::new())
        );
    }

    #[test]
    fn test_len_noarg_error() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert!(call_function("len", &[], &mut ctx).is_error());
    }

    #[test]
    fn test_substr_noarg_error() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert!(call_function("substr", &[], &mut ctx).is_error());
    }

    // === B6: Utility Edge Cases ===

    #[test]
    fn test_env_missing() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(
            call_function("$", &[Token::String("UNLIKELY_VAR_XYZ_9999".to_string())], &mut ctx),
            Token::String(String::new())
        );
    }

    #[test]
    fn test_eval_empty_cell() {
        let mut sheet = Sheet::new();
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("eval", &[Token::Location([5, 5, 0])], &mut ctx);
        assert_eq!(result, Token::Empty);
    }

    #[test]
    fn test_eval_depth_limit() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(5)]);
        sheet.update();
        let mut ctx = EvalContext { sheet: &mut sheet, x: 0, y: 0, z: 0, max_eval: 0 };
        let result = call_function("eval", &[Token::Location([0, 0, 0])], &mut ctx);
        assert!(result.is_error());
    }

    #[test]
    fn test_clock_no_args() {
        let mut sheet = Sheet::new();
        sheet.update();
        let mut ctx = make_ctx(&mut sheet);
        // No clocked value on current cell → Empty
        let result = call_function("clock", &[], &mut ctx);
        assert_eq!(result, Token::Empty);
    }

    #[test]
    fn test_strftime_with_ts() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        // Use a known timestamp: 1704067200 = 2024-01-01 00:00:00 UTC
        let result = call_function("strftime", &[Token::String("%Y".to_string()), Token::Integer(1704067200)], &mut ctx);
        match result {
            Token::String(s) => assert!(s == "2024" || s == "2023", "expected 2024 (or 2023 in far-west TZ), got: {}", s),
            _ => panic!("expected string, got {:?}", result),
        }
    }

    #[test]
    fn test_strptime_datetime() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("strptime", &[
            Token::String("%Y-%m-%d %H:%M".to_string()),
            Token::String("2024-01-15 10:30".to_string()),
        ], &mut ctx);
        match result {
            Token::Integer(ts) => assert!(ts > 0),
            other => panic!("expected integer timestamp, got: {:?}", other),
        }
    }

    // === B7: Cell Reference Edge Cases ===

    #[test]
    fn test_at_error_propagation() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        let result = call_function("@", &[Token::Error("test error".to_string())], &mut ctx);
        assert!(result.is_error());
    }

    #[test]
    fn test_x_with_location() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("x", &[Token::Location([5, 10, 0])], &mut ctx), Token::Integer(5));
    }

    #[test]
    fn test_y_with_location() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("y", &[Token::Location([5, 10, 0])], &mut ctx), Token::Integer(10));
    }

    #[test]
    fn test_z_with_location() {
        let mut sheet = Sheet::new();
        let mut ctx = make_ctx(&mut sheet);
        assert_eq!(call_function("z", &[Token::Location([5, 10, 3])], &mut ctx), Token::Integer(3));
    }
}
