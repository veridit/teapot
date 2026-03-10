# Teapot Usage Guide

## Modes

Teapot has four main modes:

- **Normal** — navigate the sheet, press keys to enter other modes
- **Editing** — type a formula or value for the current cell
- **Command** — type a `:command` to execute
- **Help** — view the help screen (press `?`)

Additionally, overlay popups can appear for the command palette, sheet picker, cell picker, and labels picker.

## Navigation

| Key | Action |
|-----|--------|
| `h` / Left | Move left |
| `j` / Down | Move down |
| `k` / Up | Move up |
| `l` / Right | Move right |
| `H` | Page left |
| `J` | Page down |
| `K` | Page up |
| `L` | Page right |
| Home | Go to cell (0,0) |
| End | Go to last cell |
| PgUp / PgDn | Page up / down |
| Tab | Next sheet |
| `[` / `]` | Previous / next sheet |
| `Z` | Sheet picker popup |

## Entering Data

| Key | Action |
|-----|--------|
| `=` | Formula entry — type an expression (e.g. `1+2`, `@(0,1,0)*2`) |
| `'` | Text entry — input is auto-wrapped in quotes, left-aligned |
| `0`-`9` | Quick number entry — starts editing with that digit |
| `e` / Enter | Edit existing cell (pre-fills current formula) |
| Esc | Cancel editing |

### Edit Mode Keys

Full readline-style editing is available in both edit and command modes:

| Key | Action |
|-----|--------|
| Left / Right | Move cursor |
| Home / End | Jump to start / end |
| Ctrl+A | Beginning of line |
| Ctrl+E | End of line |
| Ctrl+K | Kill to end of line |
| Ctrl+U | Kill to beginning of line |
| Ctrl+W | Delete word backwards |
| Ctrl+F | Forward one character |
| Ctrl+B | Back one character |
| Delete | Delete character at cursor |
| Backspace | Delete character before cursor |
| `@` | Open cell picker (in edit mode) |

### Cell Picker

When you press `@` in edit mode, a cell picker activates:
- Arrow keys move a green highlight across the sheet
- Tab cycles through sheet layers (z axis)
- Enter inserts `@(x,y,z)` at the cursor position
- Esc cancels

## Commands

Press `:` to enter command mode. Tab completes command names.

### File Operations

| Command | Description |
|---------|-------------|
| `:w [file]` | Save (auto-detects .tp/.tpz/.xlsx by extension) |
| `:o <file>` | Open file |
| `:q` | Quit (warns if unsaved changes) |
| `:q!` | Force quit |
| `:wq` | Save and quit |

### Navigation Commands

| Command | Description |
|---------|-------------|
| `:goto x,y[,z]` | Move cursor to cell |
| `:sheet N` | Switch to sheet N (1-based) |
| `:sheet-add` | Add a new sheet |
| `:sheet-del` | Delete current sheet |
| `:sheets` | Open sheet picker popup |

### Editing Commands

| Command | Description |
|---------|-------------|
| `:undo` | Undo last change |
| `:redo` | Redo last undo |
| `:yank` | Copy marked block to clipboard |
| `:paste` | Paste clipboard at cursor |

### Formatting

These commands apply to the marked block if one is set, otherwise to the current cell.

| Command | Description |
|---------|-------------|
| `:width N` | Set column width |
| `:precision N` | Set decimal precision |
| `:bold` | Toggle bold |
| `:underline` | Toggle underline |
| `:align left\|right\|center\|auto` | Set alignment |
| `:lock` | Toggle cell lock |
| `:ignore` | Toggle ignore in calculations |

### Labels

Any cell can have a named label. Labeled cells are shown with green text, and the label name appears in the status bar when the cursor is on that cell. Label references are highlighted with the same colored backgrounds as `@(x,y,z)` references.

| Command | Description |
|---------|-------------|
| `:label <name>` | Set label on current cell (clear with `:label` alone) |
| `:labels` | Open labels picker — jump to any labeled cell |

Use labels in formulas with `@("labelName")` or just `labelName` (standalone returns the value directly).

### Block Operations

Mark a block by pressing `m` at two corners. Then:

| Command | Description |
|---------|-------------|
| `:clear` | Clear all cells in block |
| `:copy` | Copy block to cursor position |
| `:move` | Move block to cursor position |
| `:sort [col] [asc\|desc]` | Sort rows by column value |
| `:sort-x [col] [asc\|desc]` | Same as :sort |
| `:sort-y [row] [asc\|desc]` | Sort columns by row value |
| `:sort-z [x,y] [asc\|desc]` | Sort layers by cell value |
| `:mirror-x` | Mirror block horizontally |
| `:mirror-y` | Mirror block vertically |
| `:mirror-z` | Mirror block across layers |
| `:fill cols rows [layers]` | Tile block in a grid pattern |

### Row/Column Operations

| Command | Description |
|---------|-------------|
| `:ir` / `:insert-row` | Insert row at cursor |
| `:dr` / `:delete-row` | Delete row at cursor |
| `:ic` / `:insert-col` | Insert column at cursor |
| `:dc` / `:delete-col` | Delete column at cursor |

### Export

If a block is marked, export commands export only the marked range. Otherwise they export the full sheet.

| Command | Description |
|---------|-------------|
| `:export-csv <file>` | Export as CSV |
| `:export-text <file>` | Export as plain text |
| `:export-html <file>` | Export as HTML |
| `:export-latex <file>` | Export as LaTeX |
| `:export-context <file>` | Export as ConTeXt |

### Clocked Cells

Clocked cells use a three-phase commit system for iterative calculations (e.g. cellular automata, simulations). Each tick: trigger -> evaluate -> commit, so all clocked cells update "simultaneously".

| Key / Command | Description |
|---------------|-------------|
| `:clock` | Toggle clock mode on current cell |
| `C` (Shift+c) | Trigger one clock tick |
| `:clock-run` | Run clock until stable (max 1000 ticks) |

## Search and Replace

Press `n` to open the search bar. Type a regex pattern — results update as you type. Press Enter or `n` again to jump to the next match. Press `N` for the previous match. Press Esc to close and clear highlights.

Matching cells are highlighted in yellow; the current match is highlighted in bright yellow.

| Key / Command | Description |
|---------------|-------------|
| `n` | Open search / next match |
| `N` | Previous match |
| `Esc` | Clear search results |
| `:search <pattern>` / `:s <pattern>` | Search current sheet |
| `:search-all <pattern>` | Search all sheets |
| `:search-formula <pattern>` | Search in formulas instead of values |
| `:replace <search> <replace>` / `:r <s> <r>` | Search and replace with confirmation |
| `:replace-all <search> <replace>` | Replace all without confirmation |

In the search bar: `Ctrl+F` toggles between searching values and formulas. `Ctrl+R` toggles regex vs literal mode. `Tab` switches focus between search and replace fields.

During replace confirmation: `y` replaces and advances, `n` skips, `a` replaces all remaining, `q` or Esc cancels.

Regex patterns support capture groups — use `$1`, `$2` etc. in the replacement string.

## Command Palette

Press `/` or `F1` to open the command palette. Type to filter commands, use Up/Down to navigate, Enter to execute, Esc to cancel.

## Command History

In command mode, use Up/Down arrows to browse previous commands.

## Normal Mode Keys (Quick Reference)

| Key | Action |
|-----|--------|
| `+` | Widen current column |
| `-` | Narrow current column |
| `m` | Set mark (press twice for block) |
| `u` | Clear mark |
| `y` | Yank (copy) marked block |
| `p` | Paste at cursor |
| `C` | Clock tick |
| `Z` | Sheet picker |
| `?` | Help screen |
| Delete | Clear current cell |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+Q | Quit |

## Batch Mode

Run with `-b` flag to read commands from stdin:

```bash
cat <<EOF | teapot -b
set 0,0 10
set 1,0 20
set 2,0 @(0,0,0)+@(1,0,0)
print 2,0
save output.csv
EOF
```

### Batch Commands

| Command | Description |
|---------|-------------|
| `goto x,y[,z]` | Move cursor |
| `from x,y[,z]` | Set mark start |
| `to x,y[,z]` | Set mark end |
| `set x,y[,z] expr` | Set cell contents |
| `print x,y[,z]` | Print cell value |
| `width col w` | Set column width |
| `precision col p` | Set column precision |
| `sort [col] [asc\|desc]` | Sort marked block by column |
| `sort-y [row] [asc\|desc]` | Sort marked block columns by row |
| `sort-z [x,y] [asc\|desc]` | Sort marked block layers |
| `mirror-x` / `mirror-y` / `mirror-z` | Mirror marked block |
| `fill cols rows [layers]` | Fill marked block |
| `bold` | Toggle bold (block-aware) |
| `underline` | Toggle underline (block-aware) |
| `lock` | Toggle lock (block-aware) |
| `ignore` | Toggle ignore (block-aware) |
| `align left\|right\|center\|auto` | Set alignment (block-aware) |
| `label <name>` | Set label on current cell |
| `clock` | Toggle clock on current cell |
| `clock-tick` | Run one clock tick |
| `search <pattern>` | Print matching cell coordinates and values |
| `replace <pattern> <replacement>` | Replace all matches (no confirmation) |
| `load <file>` | Load file |
| `load-csv <file>` | Load CSV |
| `save [file]` | Save (.tp/.tpz/.xlsx) |
| `save-csv <file>` | Save as CSV |
| `save-html <file>` | Save as HTML |
| `save-latex <file>` | Save as LaTeX |
| `save-context <file>` | Save as ConTeXt |
| `save-xlsx <file>` | Save as XLSX |
| `save-text <file>` | Save as plain text |

Save/export commands in batch mode also respect the mark range set via `from`/`to`.

## Formulas

Teapot uses a functional syntax for cell references instead of the traditional A1 notation:

```
@(x, y, z)              # absolute reference
@(x()-1, y(), z())      # relative: one column left
@(x(), y()-1, z())      # relative: one row up
@("labelName")          # label-based reference
labelName               # standalone label (returns value)
eval(@(x, y, z))        # re-evaluate a cell's formula
```

### eval()

`eval(@(x,y,z))` re-evaluates the formula in the referenced cell and returns the result. Unlike `@()` which returns the cached value, `eval()` runs the formula from scratch. This is useful for indirect references and meta-programming. Recursion depth is limited to prevent infinite loops.

### Operators (by precedence, lowest first)

| Operator | Description |
|----------|-------------|
| `<` `<=` `>=` `>` `==` `!=` `~=` | Comparison (~= is approximate equal) |
| `+` `-` | Addition, subtraction |
| `*` `/` `%` | Multiplication, division, modulo |
| `^` | Power |
| `-` (unary) | Negation |

### Built-in Functions

#### Cell References

| Function | Description |
|----------|-------------|
| `@()` | Value of current cell |
| `@(x)` | Value at column x, current row and sheet |
| `@(x, y)` | Value at (x, y), current sheet |
| `@(x, y, z)` | Value at absolute coordinates |
| `@(location)` | Value at a location token (e.g. from `min()`) |
| `@("label")` | Value of labeled cell |
| `&()` | Location of current cell |
| `&(x)` | Location (x, cur_y, cur_z) |
| `&(x, y)` | Location (x, y, cur_z) |
| `&(x, y, z)` | Location at absolute coordinates |
| `x()` / `y()` / `z()` | Current cell's x / y / z coordinate |
| `x(loc)` / `y(loc)` / `z(loc)` | Extract coordinate from a location |

Empty arguments use defaults: `@(,5,)` means `@(cur_x, 5, cur_z)`.

#### Math

| Function | Description |
|----------|-------------|
| `abs(n)` | Absolute value (preserves integer type) |
| `sin(x)` / `cos(x)` / `tan(x)` | Trigonometric functions (radians) |
| `asin(x)` / `acos(x)` / `atan(x)` | Inverse trigonometric |
| `sinh(x)` / `cosh(x)` / `tanh(x)` | Hyperbolic |
| `arsinh(x)` / `arcosh(x)` / `artanh(x)` | Inverse hyperbolic |
| `deg2rad(x)` / `rad2deg(x)` | Angle conversion |
| `log(x)` | Natural logarithm |
| `log(x, base)` | Logarithm with specified base |
| `e()` | Euler's number (2.718…) |
| `e(x)` | e raised to power x |
| `rnd()` | Random float in [0, 1) |
| `poly(x, c0, c1, …)` | Polynomial: c0 + c1·x + c2·x² + … |

#### String

| Function | Description |
|----------|-------------|
| `len(s)` | Length of string |
| `substr(s, start, end)` | Substring from index `start` to `end` (inclusive, 0-based) |

#### Aggregates

All aggregate functions take two location arguments defining a range.

| Function | Description |
|----------|-------------|
| `sum(loc1, loc2)` | Sum of all values in range |
| `n(loc1, loc2)` | Count of non-empty cells in range |
| `min(loc1, loc2)` | **Location** of the cell with the minimum value |
| `max(loc1, loc2)` | **Location** of the cell with the maximum value |

`min()` and `max()` return a location, not a value. Use `@(min(…))` to get the value, or `x(min(…))` to get its column.

#### Type Conversion

| Function | Description |
|----------|-------------|
| `int(n)` | Convert float to integer (truncates toward zero) |
| `int("s")` | Parse string as integer |
| `int(n, neg_mode, pos_mode)` | Rounding control — mode: <−1 floor, −1 away from zero, 0 truncate, 1 toward zero, >1 ceil |
| `float(n)` | Convert to float |
| `frac(n)` | Fractional part (e.g. `frac(3.14)` = 0.14) |
| `string(n)` | Convert to string |
| `string(n, precision)` | Format with decimal places (e.g. `string(3.14159, 2)` = "3.14") |
| `string(n, precision, mode)` | Mode 0 = fixed, non-zero = scientific notation |
| `error("msg")` | Create an error value |

#### Utility

| Function | Description |
|----------|-------------|
| `eval(location)` | Re-evaluate a cell's formula (not cached value) |
| `clock()` | Read clocked value of current cell |
| `clock(init)` | Read clocked value, using init as default |
| `clock(cond, loc)` | If cond ≠ 0, enable clock on cell at loc |
| `clock(cond, loc1, loc2)` | Enable clock on two cells |
| `$("NAME")` | Read environment variable |
| `time()` | Current unix timestamp (integer) |
| `strftime(fmt)` | Format current time (e.g. `strftime("%Y-%m-%d")`) |
| `strftime(fmt, timestamp)` | Format given unix timestamp |
| `strptime(fmt, string)` | Parse time string → unix timestamp |
