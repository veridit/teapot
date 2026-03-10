# Teapot — Status and Roadmap

## Completed

### Core Engine
- [x] Token, Scanner, Parser (recursive descent evaluator, no AST)
- [x] 41 built-in functions (math, string, cell refs, aggregates, utility)
- [x] 3D sparse HashMap-based sheet with lazy on-demand evaluation
- [x] Type-safe arithmetic with promotion rules matching C original

### File I/O
- [x] Portable ASCII format (.tp) — load/save
- [x] Compressed format (.tpz) — load/save
- [x] CSV — load/save
- [x] XLSX — load/save (via calamine/rust_xlsxwriter)
- [x] ODS/XLS — load (via calamine)
- [x] HTML export
- [x] LaTeX export
- [x] ConTeXt export
- [x] Plain text export

### Terminal UI (ratatui/crossterm)
- [x] Spreadsheet grid with column/row headers
- [x] Sheet tabs with navigation
- [x] Cell editing with formula pre-fill
- [x] vim-style navigation (hjkl, HJKL page, Home/End, PgUp/PgDn)
- [x] Command mode (: prefix)
- [x] Block operations: mark, copy, move, clear, sort, yank/paste
- [x] Insert/delete rows and columns
- [x] Column width adjustment (+/-)
- [x] Cell formatting: alignment, precision, bold, underline
- [x] Undo/redo (50 levels)
- [x] Batch mode (stdin commands)
- [x] Help screen (?)

### Modern TUI Features
- [x] Smart entry keys: = for formula, ' for text (auto-quotes), 0-9 for numbers
- [x] Full readline-style editing (Left/Right, Home/End, Ctrl+A/E/K/U/W/F/B, Delete)
- [x] Command history (Up/Down, max 100 entries)
- [x] Tab completion for commands
- [x] @ cell picker (highlight on sheet, insert reference)
- [x] Sheet navigation: [/] keys, Z picker popup, :sheet/:sheet-add/:sheet-del
- [x] Mirror block (:mirror-x/y/z)
- [x] Fill/tile block (:fill cols rows [layers])
- [x] Multi-axis sort (:sort-x, :sort-y, :sort-z)
- [x] Clocked cells — three-phase commit (C key, :clock, :clock-run)
- [x] Command palette (/ or F1) with fuzzy filtering

## Remaining Work

### File Formats
- [ ] XDR binary format (original native format) — load/save
- [ ] SC (Spreadsheet Calculator) format — load
- [ ] WK1 (Lotus 1-2-3) format — load

### UI Polish
- [ ] Mouse support (click to select cell, scroll)
- [ ] Theming / color schemes
- [ ] Resizable column widths by dragging
- [ ] Status bar: show mark range, modified indicator
- [ ] Configurable key bindings

### Missing C Features
- [ ] Internationalization (message catalogs)
- [ ] Label-based cell references in formulas (@(NamedCell))
- [ ] eval() function for formula references
- [ ] Block-scoped formatting operations
- [ ] Print/export marked block only

### Future Ideas
- [ ] Undo descriptions (show what each undo step reverts)
- [ ] Search/replace within cells
- [ ] Conditional formatting
- [ ] Charts (sparklines in terminal)
- [ ] Lua/Rhai scripting extension
- [ ] Web-based viewer
