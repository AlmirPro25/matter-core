# Change Log

All notable changes to the Matter Language extension will be documented in this file.

## [0.7.0] - 2026-05-09

### Added
- Initial release of Matter Language extension
- Syntax highlighting for Matter language
  - Keywords: let, set, fn, if, else, while, loop, for, break, continue, return, on, import
  - Types: int, bool, string, unit, list, map, struct
  - Backend calls: agent, visual, store, net, math, string, list, time, random, json
  - Comments, strings, numbers, operators
- Language Server Protocol (LSP) integration
  - Diagnostics (real-time error detection)
  - Autocomplete (variables, functions, backends, keywords)
  - Go-to-definition
  - Hover information
  - Find references
  - Rename symbol
  - Document symbols
- Code snippets
  - Function declaration (fn)
  - Control flow (if, while, for, loop)
  - Event handlers (on)
  - Import statements
  - Backend calls
- Commands
  - Matter: Run File
  - Matter: Compile File
  - Matter: Run Bytecode
  - Matter: Format File
  - Matter: Lint File
  - Matter: Debug File
  - Matter: Show Backends
  - Matter: Show Examples
- Formatter integration
  - Automatic code formatting
  - Format on save support
- Linter integration
  - Unused variable detection
  - Unused function detection
  - Code quality analysis
- File icons for .matter and .mbc files
- Configuration options
  - Enable/disable LSP
  - Configure matter-cli path
  - Enable/disable formatter
  - Enable/disable linter
  - LSP trace level
- Context menu integration
- Editor title menu integration
- Language configuration
  - Auto-closing pairs
  - Bracket matching
  - Comment toggling
  - Indentation rules

### Features
- Full LSP support for professional IDE experience
- Real-time error detection and feedback
- Intelligent autocomplete with backend method suggestions
- Code navigation (go-to-definition, find references)
- Symbol renaming across files
- Document outline view
- Integrated debugging support
- One-click file execution
- Automatic code formatting
- Static code analysis

### Requirements
- Matter CLI (matter-cli) must be installed
- VS Code 1.75.0 or higher

### Known Issues
- None

---

## Future Releases

### [0.8.0] - Planned
- Semantic syntax highlighting
- Advanced autocomplete (context-aware)
- Code actions (quick fixes)
- Refactoring tools
- Improved error messages
- Performance optimizations

### [0.9.0] - Planned
- Visual debugger integration
- Breakpoint support in editor
- Variable inspection in hover
- Call stack visualization
- Test runner integration
- Code coverage visualization

### [1.0.0] - Planned
- Stable API
- Complete documentation
- Tutorial integration
- Example browser
- Package manager integration
- Remote development support
