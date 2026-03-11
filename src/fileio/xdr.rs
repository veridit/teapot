//! XDR binary file format handler (load only)
//!
//! XDR is C teapot's native binary format. This module provides load support
//! so users can migrate existing .xdr files to the Rust implementation.

use anyhow::{bail, Result};
use std::io::Read;

use crate::sheet::{Adjust, Sheet};
use crate::token::{Operator, Token};

/// Function identifier index → name mapping (from C src/func.c:1369 tfunc[] table)
const FIDENT_TABLE: &[&str] = &[
    "@", "&", "x", "y", "z", "eval", "error", "string", "sum", "n",
    "int", "frac", "len", "min", "max", "abs", "$", "float",
    "strftime", "clock", "poly", "e", "log", "sin", "cos", "tan",
    "sinh", "cosh", "tanh", "asin", "acos", "atan", "arsinh",
    "arcosh", "artanh", "deg2rad", "rad2deg", "rnd", "substr",
    "strptime", "time",
];

/// Operator index → Operator mapping (C operator encoding)
const OPERATOR_TABLE: &[Operator] = &[
    Operator::Plus,       // 0
    Operator::Minus,      // 1
    Operator::Mul,        // 2
    Operator::Div,        // 3
    Operator::OpenParen,  // 4
    Operator::CloseParen, // 5
    Operator::Comma,      // 6
    Operator::Lt,         // 7
    Operator::Le,         // 8
    Operator::Ge,         // 9
    Operator::Gt,         // 10
    Operator::Equal,      // 11
    Operator::AboutEqual, // 12
    Operator::NotEqual,   // 13
    Operator::Pow,        // 14
    Operator::Mod,        // 15
];

/// C token type constants
const TYPE_EMPTY: i32 = 0;
const TYPE_STRING: i32 = 1;
const TYPE_FLOAT: i32 = 2;
const TYPE_INT: i32 = 3;
const TYPE_OPERATOR: i32 = 4;
const TYPE_LIDENT: i32 = 5;
const TYPE_FIDENT: i32 = 6;
const TYPE_LOCATION: i32 = 7;
const TYPE_EEK: i32 = 8;

fn read_xdr_int(r: &mut impl Read) -> Result<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_be_bytes(buf))
}

fn read_xdr_double(r: &mut impl Read) -> Result<f64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(f64::from_be_bytes(buf))
}

fn read_xdr_string(r: &mut impl Read) -> Result<String> {
    let len = read_xdr_int(r)? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    // XDR strings are padded to 4-byte boundary
    let pad = (4 - (len % 4)) % 4;
    if pad > 0 {
        let mut pad_buf = vec![0u8; pad];
        r.read_exact(&mut pad_buf)?;
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn read_xdr_token(r: &mut impl Read) -> Result<Token> {
    let type_int = read_xdr_int(r)?;
    // Operator is encoded in upper bits: type = OPERATOR | (op_index << 8)
    let base_type = type_int & 0xFF;
    let op_index = ((type_int >> 8) & 0xFF) as usize;

    match base_type {
        TYPE_EMPTY => Ok(Token::Empty),
        TYPE_STRING => {
            let s = read_xdr_string(r)?;
            Ok(Token::String(s))
        }
        TYPE_FLOAT => {
            let f = read_xdr_double(r)?;
            Ok(Token::Float(f))
        }
        TYPE_INT => {
            let i = read_xdr_int(r)? as i64;
            Ok(Token::Integer(i))
        }
        TYPE_OPERATOR => {
            if op_index < OPERATOR_TABLE.len() {
                Ok(Token::Operator(OPERATOR_TABLE[op_index]))
            } else {
                bail!("Unknown operator index {}", op_index)
            }
        }
        TYPE_LIDENT => {
            let s = read_xdr_string(r)?;
            Ok(Token::LabelIdentifier(s))
        }
        TYPE_FIDENT => {
            let idx = read_xdr_int(r)? as usize;
            if idx < FIDENT_TABLE.len() {
                Ok(Token::Identifier(FIDENT_TABLE[idx].to_string()))
            } else {
                bail!("Unknown function index {}", idx)
            }
        }
        TYPE_LOCATION => {
            let x = read_xdr_int(r)? as usize;
            let y = read_xdr_int(r)? as usize;
            let z = read_xdr_int(r)? as usize;
            Ok(Token::Location([x, y, z]))
        }
        TYPE_EEK => {
            let s = read_xdr_string(r)?;
            Ok(Token::Error(s))
        }
        _ => bail!("Unknown token type {}", base_type),
    }
}

/// Read a token array with XDR pointer indirection.
/// Format: int(count), then for each: int(pointer != 0 flag), token data
fn read_xdr_token_vec(r: &mut impl Read) -> Result<Option<Vec<Token>>> {
    let count = read_xdr_int(r)?;
    if count == 0 {
        return Ok(None);
    }
    let mut tokens = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let ptr = read_xdr_int(r)?;
        if ptr != 0 {
            tokens.push(read_xdr_token(r)?);
        }
    }
    if tokens.is_empty() {
        Ok(None)
    } else {
        Ok(Some(tokens))
    }
}

/// Read a cell from XDR format
fn read_xdr_cell(r: &mut impl Read, sheet: &mut Sheet, x: usize, y: usize, z: usize) -> Result<()> {
    // contents
    let contents = read_xdr_token_vec(r)?;
    // ccontents (clocked contents)
    let ccontents = read_xdr_token_vec(r)?;

    // label — discriminated union: 0=null, 1=string
    let has_label = read_xdr_int(r)?;
    let label = if has_label != 0 {
        Some(read_xdr_string(r)?)
    } else {
        None
    };

    // adjust — int enum
    let adjust_int = read_xdr_int(r)?;
    let adjust = match adjust_int {
        0 => Adjust::Right,
        1 => Adjust::Left,
        2 => Adjust::Center,
        _ => Adjust::AutoAdjust,
    };

    // precision
    let precision = read_xdr_int(r)?;

    // flags — single int with 8 bits packed
    let flags = read_xdr_int(r)?;
    let updated = flags & (1 << 0) != 0;
    let shadowed = flags & (1 << 1) != 0;
    let scientific = flags & (1 << 2) != 0;
    let locked = flags & (1 << 3) != 0;
    let transparent = flags & (1 << 4) != 0;
    let ignored = flags & (1 << 5) != 0;
    let bold = flags & (1 << 6) != 0;
    let underline = flags & (1 << 7) != 0;

    let cell = sheet.get_or_create_cell(x, y, z);
    cell.contents = contents;
    cell.clocked_contents = ccontents;
    cell.label = label;
    cell.adjust = adjust;
    cell.precision = precision;
    cell.updated = updated;
    cell.shadowed = shadowed;
    cell.scientific = scientific;
    cell.locked = locked;
    cell.transparent = transparent;
    cell.ignored = ignored;
    cell.bold = bold;
    cell.underline = underline;

    // Grow dimensions
    if x >= sheet.dim_x { sheet.dim_x = x + 1; }
    if y >= sheet.dim_y { sheet.dim_y = y + 1; }
    if z >= sheet.dim_z { sheet.dim_z = z + 1; }

    Ok(())
}

/// Load a sheet from an XDR binary file
pub fn load_xdr(sheet: &mut Sheet, filename: &str) -> Result<()> {
    let file = std::fs::File::open(filename)?;
    let mut r = std::io::BufReader::new(file);

    // Read magic: 3 × xdr_int: "#!te", "apot", "\nxdr"
    let magic1 = read_xdr_int(&mut r)?;
    let magic2 = read_xdr_int(&mut r)?;
    let magic3 = read_xdr_int(&mut r)?;

    let expected1 = i32::from_be_bytes(*b"#!te");
    let expected2 = i32::from_be_bytes(*b"apot");
    let expected3 = i32::from_be_bytes(*b"\nxdr");

    if magic1 != expected1 || magic2 != expected2 || magic3 != expected3 {
        bail!("Not an XDR teapot file (bad magic)");
    }

    // Read records until EOF
    loop {
        let record_type = match read_xdr_int(&mut r) {
            Ok(t) => t,
            Err(_) => break, // EOF
        };

        match record_type {
            0 => {
                // Column width record: x, z, width
                let x = read_xdr_int(&mut r)? as usize;
                let z = read_xdr_int(&mut r)? as usize;
                let width = read_xdr_int(&mut r)? as usize;
                sheet.set_width(x, z, width);
            }
            1 => {
                // Cell record: x, y, z, then cell data
                let x = read_xdr_int(&mut r)? as usize;
                let y = read_xdr_int(&mut r)? as usize;
                let z = read_xdr_int(&mut r)? as usize;
                read_xdr_cell(&mut r, sheet, x, y, z)?;
            }
            _ => bail!("Unknown XDR record type {}", record_type),
        }
    }

    sheet.changed = false;
    sheet.cachelabels();
    sheet.update();
    Ok(())
}

/// Save a sheet to an XDR file — not supported, use .tpa or .tpz instead
pub fn save_xdr(_sheet: &Sheet, _filename: &str) -> Result<usize> {
    bail!("XDR save not supported. Use save (.tpa) or save-tpz (.tpz) instead.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Build XDR bytes in memory for testing
    fn build_test_xdr() -> Vec<u8> {
        let mut buf = Vec::new();

        // Magic
        buf.extend_from_slice(&i32::to_be_bytes(i32::from_be_bytes(*b"#!te")));
        buf.extend_from_slice(&i32::to_be_bytes(i32::from_be_bytes(*b"apot")));
        buf.extend_from_slice(&i32::to_be_bytes(i32::from_be_bytes(*b"\nxdr")));

        // Column width record (type=0): x=1, z=0, width=15
        buf.extend_from_slice(&0i32.to_be_bytes()); // record type 0
        buf.extend_from_slice(&1i32.to_be_bytes()); // x
        buf.extend_from_slice(&0i32.to_be_bytes()); // z
        buf.extend_from_slice(&15i32.to_be_bytes()); // width

        // Cell record (type=1): x=0, y=0, z=0, containing Integer(42)
        buf.extend_from_slice(&1i32.to_be_bytes()); // record type 1
        buf.extend_from_slice(&0i32.to_be_bytes()); // x
        buf.extend_from_slice(&0i32.to_be_bytes()); // y
        buf.extend_from_slice(&0i32.to_be_bytes()); // z

        // Cell data:
        // contents: 1 token (Integer 42)
        buf.extend_from_slice(&1i32.to_be_bytes()); // count=1
        buf.extend_from_slice(&1i32.to_be_bytes()); // ptr!=0
        buf.extend_from_slice(&TYPE_INT.to_be_bytes()); // type=INT
        buf.extend_from_slice(&42i32.to_be_bytes()); // value=42

        // ccontents: 0 tokens
        buf.extend_from_slice(&0i32.to_be_bytes()); // count=0

        // label: none
        buf.extend_from_slice(&0i32.to_be_bytes()); // has_label=0

        // adjust: Left (1)
        buf.extend_from_slice(&1i32.to_be_bytes());

        // precision: 2
        buf.extend_from_slice(&2i32.to_be_bytes());

        // flags: bold(64) | locked(8) = 72
        buf.extend_from_slice(&72i32.to_be_bytes());

        // Cell record (type=1): x=1, y=0, z=0, containing String("hello")
        buf.extend_from_slice(&1i32.to_be_bytes()); // record type 1
        buf.extend_from_slice(&1i32.to_be_bytes()); // x
        buf.extend_from_slice(&0i32.to_be_bytes()); // y
        buf.extend_from_slice(&0i32.to_be_bytes()); // z

        // contents: 1 token (String "hello")
        buf.extend_from_slice(&1i32.to_be_bytes()); // count=1
        buf.extend_from_slice(&1i32.to_be_bytes()); // ptr!=0
        buf.extend_from_slice(&TYPE_STRING.to_be_bytes()); // type=STRING
        buf.extend_from_slice(&5i32.to_be_bytes()); // string length=5
        buf.extend_from_slice(b"hello");
        buf.extend_from_slice(&[0, 0, 0]); // padding to 4-byte boundary

        // ccontents: 0
        buf.extend_from_slice(&0i32.to_be_bytes());
        // label: none
        buf.extend_from_slice(&0i32.to_be_bytes());
        // adjust: AutoAdjust (3)
        buf.extend_from_slice(&3i32.to_be_bytes());
        // precision: -1
        buf.extend_from_slice(&(-1i32).to_be_bytes());
        // flags: 0
        buf.extend_from_slice(&0i32.to_be_bytes());

        // Cell with Float and function identifier
        buf.extend_from_slice(&1i32.to_be_bytes()); // record type 1
        buf.extend_from_slice(&0i32.to_be_bytes()); // x
        buf.extend_from_slice(&1i32.to_be_bytes()); // y
        buf.extend_from_slice(&0i32.to_be_bytes()); // z

        // contents: 4 tokens: sin ( 3.14 )
        buf.extend_from_slice(&4i32.to_be_bytes()); // count=4
        // FIDENT sin (index 23)
        buf.extend_from_slice(&1i32.to_be_bytes()); // ptr
        buf.extend_from_slice(&TYPE_FIDENT.to_be_bytes());
        buf.extend_from_slice(&23i32.to_be_bytes()); // sin
        // OpenParen
        buf.extend_from_slice(&1i32.to_be_bytes()); // ptr
        let op_type = TYPE_OPERATOR | (4 << 8); // OpenParen index=4
        buf.extend_from_slice(&op_type.to_be_bytes());
        // Float 3.14
        buf.extend_from_slice(&1i32.to_be_bytes()); // ptr
        buf.extend_from_slice(&TYPE_FLOAT.to_be_bytes());
        buf.extend_from_slice(&3.14f64.to_be_bytes());
        // CloseParen
        buf.extend_from_slice(&1i32.to_be_bytes()); // ptr
        let cp_type = TYPE_OPERATOR | (5 << 8); // CloseParen index=5
        buf.extend_from_slice(&cp_type.to_be_bytes());

        // ccontents: 0
        buf.extend_from_slice(&0i32.to_be_bytes());
        // label: "test_label"
        buf.extend_from_slice(&1i32.to_be_bytes()); // has_label=1
        buf.extend_from_slice(&10i32.to_be_bytes()); // len=10
        buf.extend_from_slice(b"test_label");
        buf.extend_from_slice(&[0, 0]); // pad to 12 bytes
        // adjust: Right (0)
        buf.extend_from_slice(&0i32.to_be_bytes());
        // precision: -1
        buf.extend_from_slice(&(-1i32).to_be_bytes());
        // flags: shadowed(2) | underline(128) = 130
        buf.extend_from_slice(&130i32.to_be_bytes());

        buf
    }

    #[test]
    fn test_xdr_load_constructed() {
        let xdr_data = build_test_xdr();

        // Write to temp file
        let tmpfile = "/tmp/teapot_test_xdr_load.xdr";
        {
            let mut f = std::fs::File::create(tmpfile).unwrap();
            f.write_all(&xdr_data).unwrap();
        }

        let mut sheet = Sheet::new();
        load_xdr(&mut sheet, tmpfile).unwrap();

        // Check column width
        assert_eq!(sheet.column_width(1, 0), 15);

        // Check cell (0,0,0): Integer(42), Left, precision=2, bold+locked
        let cell00 = sheet.get_cell(0, 0, 0).unwrap();
        assert_eq!(cell00.value, Token::Integer(42));
        assert_eq!(cell00.adjust, Adjust::Left);
        assert_eq!(cell00.precision, 2);
        assert!(cell00.bold);
        assert!(cell00.locked);

        // Check cell (1,0,0): String("hello")
        let cell10 = sheet.get_cell(1, 0, 0).unwrap();
        assert_eq!(cell10.value, Token::String("hello".to_string()));
        assert_eq!(cell10.precision, -1);

        // Check cell (0,1,0): has sin function, label, shadowed+underline
        let cell01 = sheet.get_cell(0, 1, 0).unwrap();
        assert!(cell01.contents.is_some());
        let contents = cell01.contents.as_ref().unwrap();
        assert_eq!(contents[0], Token::Identifier("sin".to_string()));
        assert_eq!(contents[1], Token::Operator(Operator::OpenParen));
        assert_eq!(contents[3], Token::Operator(Operator::CloseParen));
        assert_eq!(cell01.label, Some("test_label".to_string()));
        assert_eq!(cell01.adjust, Adjust::Right);
        assert!(cell01.shadowed);
        assert!(cell01.underline);

        std::fs::remove_file(tmpfile).ok();
    }

    #[test]
    fn test_xdr_save_stub_error() {
        let sheet = Sheet::new();
        let result = save_xdr(&sheet, "/tmp/test.xdr");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("not supported"), "got: {}", msg);
    }

    #[test]
    fn test_xdr_bad_magic() {
        let tmpfile = "/tmp/teapot_test_xdr_bad_magic.xdr";
        {
            let mut f = std::fs::File::create(tmpfile).unwrap();
            f.write_all(&[0u8; 12]).unwrap();
        }
        let mut sheet = Sheet::new();
        let result = load_xdr(&mut sheet, tmpfile);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bad magic"));
        std::fs::remove_file(tmpfile).ok();
    }
}
