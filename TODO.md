# Teapot Modernization Project: Rust Rewrite

## Overview
This document outlines a detailed, incremental plan to port the Teapot spreadsheet application from C to Rust. Teapot is a terminal-based spreadsheet with features like cell editing, formatting, sorting, and various file format support.

## Why Rust?
- Strong memory safety guarantees without garbage collection
- Pattern matching and algebraic data types for handling cell values
- Good FFI capabilities for interfacing with C libraries during transition
- Excellent error handling with Result/Option types
- Performance comparable to C
- Strong type system to prevent bugs at compile time
- Growing ecosystem for terminal UI applications

## Notes
* The old build system uses cmake, the rust build must be compatible.
* Write out the Rust source code in the src/ folder directly, matching the C files.

## Incremental Rewrite Strategy

The rewrite will follow an incremental approach where we:
1. Start with a working subset of functionality
2. Gradually replace C components with Rust
3. Maintain compatibility with existing file formats throughout
4. Allow for a hybrid C/Rust application during transition

### Phase 0: Project Setup and Analysis (2 weeks)
- **Week 1: Setup and Initial Analysis**
  - Create Rust project structure with Cargo
  - Set up CI/CD pipeline
  - Define FFI boundaries for C/Rust interop
  - Create detailed component dependency graph
  - Document key data structures and algorithms

- **Week 2: Proof of Concept**
  - Implement minimal Sheet and Cell structures in Rust
  - Create FFI bindings to test Rust/C interoperability
  - Implement simple file I/O to validate approach
  - Benchmark performance against C implementation

### Phase 1: Core Data Model (4 weeks)
- **Week 1-2: Basic Data Structures**
  - Implement Sheet structure with ownership semantics
  - Implement Cell representation with variants for different types
  - Create Token enum for expression representation
  - Implement basic operations (get/set cell values)
  - Write comprehensive tests for data structures

- **Week 3-4: Memory Management**
  - Implement efficient memory management strategy
  - Create safe wrappers around any unsafe code
  - Optimize for performance while maintaining safety
  - Benchmark against original implementation

**Milestone 1:** Rust library that can create, modify, and store basic sheet data

### Phase 2: Expression Evaluation (6 weeks)
- **Week 1-2: Scanner/Tokenizer**
  - Implement lexical analyzer for expressions
  - Create token stream representation
  - Handle all token types from original implementation
  - Write tests for tokenization edge cases

- **Week 3-4: Parser**
  - Implement recursive descent parser
  - Create AST representation
  - Handle operator precedence and associativity
  - Implement error recovery strategies
  - Write tests for parsing complex expressions

- **Week 5-6: Evaluator**
  - Implement expression evaluator
  - Port built-in functions
  - Handle type conversions safely
  - Implement cell reference resolution
  - Create comprehensive test suite for evaluation

**Milestone 2:** Library that can parse and evaluate expressions

### Phase 3: File I/O (3 weeks)
- **Week 1: Native Format**
  - Implement loading/saving in native format (XDR)
  - Ensure compatibility with existing files
  - Create migration path for legacy files

- **Week 2: Import Formats**
  - Implement CSV import
  - Implement SC import
  - Implement WK1 import

- **Week 3: Export Formats**
  - Implement HTML export
  - Implement LaTeX export
  - Implement text/CSV export

**Milestone 3:** Complete file compatibility with original Teapot

### Phase 4: Terminal UI (4 weeks)
- **Week 1: UI Framework**
  - Evaluate and select terminal UI library (crossterm + ratatui)
  - Implement basic UI layout
  - Create event handling system

- **Week 2: Cell Display**
  - Implement cell rendering
  - Handle formatting and styling
  - Implement scrolling and navigation

- **Week 3: Editing Interface**
  - Implement cell editing
  - Create command input interface
  - Implement keyboard shortcuts

- **Week 4: Menus and Help**
  - Implement menu system
  - Create context-sensitive help
  - Add status messages and error display

**Milestone 4:** Functional terminal UI application

### Phase 5: Testing and Refinement (3 weeks)
- **Week 1: Integration Testing**
  - Create end-to-end tests
  - Verify compatibility with original behavior
  - Benchmark performance

- **Week 2: Optimization**
  - Profile application performance
  - Optimize critical paths
  - Reduce memory usage

- **Week 3: Documentation and Packaging**
  - Complete user documentation
  - Create installation packages
  - Prepare for release

**Milestone 5:** Production-ready application

## Hybrid Approach During Transition

To maintain a working application throughout the rewrite:

1. **FFI Bridge Layer**
   - Create a Rust-C FFI layer to allow gradual replacement of components
   - Use `extern "C"` functions to expose Rust functionality to C
   - Use `bindgen` to generate Rust bindings for C code

2. **Component Isolation**
   - Identify components that can be replaced independently
   - Create clean interfaces between components
   - Replace components one at a time while maintaining functionality

3. **Parallel Implementations**
   - Maintain both C and Rust implementations during transition
   - Add feature flags to switch between implementations
   - Use automated tests to verify equivalent behavior

## Technical Implementation Details

### Data Structures

```rust
// Core data structures
enum CellValue {
    Empty,
    Integer(i64),
    Float(f64),
    String(String),
    Error(String),
    CellRef(CellAddress),
}

struct Cell {
    value: CellValue,
    formula: Option<String>,
    format: CellFormat,
    // Additional properties
}

struct Sheet {
    cells: HashMap<CellAddress, Cell>,
    column_widths: HashMap<usize, usize>,
    // Additional properties
}

// Expression evaluation
enum Token {
    Integer(i64),
    Float(f64),
    String(String),
    Identifier(String),
    Operator(Operator),
    Function(String),
    // Other token types
}

enum Operator {
    Add, Subtract, Multiply, Divide, Power,
    Equal, NotEqual, LessThan, GreaterThan,
    // Other operators
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

// UI Components
struct TeapotApp {
    sheet: Sheet,
    cursor: CellAddress,
    view_offset: (usize, usize),
    mode: AppMode,
    // Additional state
}

enum AppMode {
    Normal,
    Edit,
    Command,
    Menu,
}
```

### Key Libraries to Use

1. **Terminal UI**
   - `crossterm` for terminal control
   - `ratatui` for widgets and layout (successor to tui-rs)

2. **Parsing**
   - `nom` or `pest` for expression parsing
   - Custom recursive descent parser as needed

3. **Serialization**
   - `serde` for data serialization/deserialization
   - Custom format handlers for legacy formats

4. **Testing**
   - `proptest` for property-based testing
   - Standard Rust test framework

5. **FFI**
   - `bindgen` for generating Rust bindings to C
   - `cbindgen` for generating C headers from Rust

## Compatibility Considerations

1. **File Format Compatibility**
   - Maintain 100% compatibility with existing Teapot files
   - Support all import/export formats from original

2. **User Experience**
   - Preserve keyboard shortcuts and commands
   - Maintain familiar UI layout
   - Add new features in non-disruptive way

3. **Performance**
   - Ensure performance is equal or better than C version
   - Optimize memory usage for large spreadsheets

## Future Enhancements

After completing the basic port:

1. **Modern UI Improvements**
   - Add themes and styling options
   - Implement mouse support
   - Add optional GUI frontend using egui or iced

2. **Enhanced Functionality**
   - Add more powerful formula capabilities
   - Implement charts and visualizations
   - Add macro/scripting support

3. **Collaboration Features**
   - Implement file locking for shared access
   - Add optional network collaboration
   - Create web-based viewer

## Conclusion

This incremental approach to rewriting Teapot in Rust allows for:
- Continuous functionality throughout the transition
- Early detection of design issues
- Gradual learning and adaptation
- Maintaining compatibility with existing files and user expectations

The end result will be a modern, memory-safe implementation that preserves the simplicity and power of the original Teapot while adding the safety and expressiveness of Rust.
