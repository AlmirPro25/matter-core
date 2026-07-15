# Matter Language Extension for VS Code

Official VS Code extension for Matter Core - A runtime-oriented language system.

## Features

### Syntax Highlighting
- Keywords: `let`, `set`, `fn`, `if`, `else`, `while`, `loop`, `for`, `break`, `continue`, `return`, `on`, `import`
- Types: `int`, `bool`, `string`, `unit`, `list`, `map`, `struct`
- Backend calls: `agent.say()`, `visual.run()`, `store.set()`, etc
- Comments, strings, numbers, operators

### Language Server Protocol (LSP)
- **Diagnostics**: Real-time error detection
- **Autocomplete**: Variables, functions, backends, keywords
- **Go-to-Definition**: Navigate to symbol definitions
- **Hover Information**: View symbol information
- **Find References**: Find all uses of a symbol
- **Rename Symbol**: Rename across all files
- **Document Symbols**: Outline view of functions and variables

### Code Snippets
- `fn` - Function declaration
- `if` - If statement
- `while` - While loop
- `for` - For loop
- `on` - Event handler
- `import` - Import statement
- Backend snippets: `agent.say`, `visual.run`, `store.set`, etc

### Commands
- **Matter: Run File** - Execute current Matter file
- **Matter: Compile File** - Compile to bytecode (.mbc)
- **Matter: Run Bytecode** - Execute bytecode file
- **Matter: Format File** - Format code automatically
- **Matter: Lint File** - Analyze code for issues
- **Matter: Debug File** - Start interactive debugger
- **Matter: Show Backends** - List available backends
- **Matter: Show Examples** - Show example files

### Formatting
- Automatic code formatting
- Consistent indentation
- Proper spacing
- Format on save (configurable)

### Linting
- Unused variable detection
- Unused function detection
- Code quality analysis
- Configurable severity levels

## Requirements

- **Matter CLI** must be installed and available in PATH
- Install Matter Core from: https://github.com/matter-core/matter-core

## Installation

### From VS Code Marketplace
1. Open VS Code
2. Go to Extensions (Ctrl+Shift+X)
3. Search for "Matter Language"
4. Click Install

### From VSIX
1. Download the `.vsix` file
2. Open VS Code
3. Go to Extensions (Ctrl+Shift+X)
4. Click "..." menu → "Install from VSIX..."
5. Select the downloaded file

### From Source
```bash
cd vscode-extension
npm install
npm run compile
code --install-extension matter-0.7.0.vsix
```

## Configuration

### Settings

```json
{
  // Enable/disable LSP
  "matter.lsp.enabled": true,

  // Path to matter-lsp.exe (empty = auto-discover). Not matter-cli.
  "matter.lsp.path": "",

  // Path to language-only CLI for Run/Compile commands
  "matter.cli.path": "matter-cli",

  // Enable/disable formatter
  "matter.formatter.enabled": true,

  // Enable/disable linter
  "matter.linter.enabled": true,

  // Enable/disable debugger
  "matter.debug.enabled": true,

  // LSP trace level (off, messages, verbose)
  "matter.trace.server": "off"
}
```

### Keybindings

Add custom keybindings in `keybindings.json`:

```json
[
  {
    "key": "ctrl+shift+r",
    "command": "matter.runFile",
    "when": "editorLangId == matter"
  },
  {
    "key": "ctrl+shift+f",
    "command": "matter.formatFile",
    "when": "editorLangId == matter"
  }
]
```

## Usage

### Running Matter Files

1. Open a `.matter` file
2. Press `Ctrl+Shift+P`
3. Type "Matter: Run File"
4. Or right-click in editor → "Matter: Run File"

### Formatting Code

1. Open a `.matter` file
2. Press `Ctrl+Shift+P`
3. Type "Matter: Format File"
4. Or use Format Document (Shift+Alt+F)

### Debugging

1. Open a `.matter` file
2. Press `Ctrl+Shift+P`
3. Type "Matter: Debug File"
4. Interactive debugger opens in terminal

### Using Autocomplete

Type and press `Ctrl+Space`:
- Variable names
- Function names
- Backend methods (type `agent.` and see suggestions)
- Keywords

### Using Snippets

Type snippet prefix and press `Tab`:
- `fn` → Function declaration
- `if` → If statement
- `for` → For loop
- `on` → Event handler

## Examples

### Hello World
```matter
print "Hello, Matter!"
```

### Function with Recursion
```matter
fn fatorial(n) {
    if n <= 1 { return 1 }
    return n * fatorial(n - 1)
}

print fatorial(5)  # 120
```

### Event Handler
```matter
on boot {
    print "System started"
    agent.say("Hello from Matter!")
}
```

### Backend Integration
```matter
# Store data
store.set("counter", 0)

# Get data
let count = store.get("counter")
set count = count + 1
store.set("counter", count)

# HTTP request
let response = net.request("https://api.example.com/data")
print response
```

## Troubleshooting

### LSP Not Working

1. Check that `matter-cli` is in PATH:
   ```bash
   matter-cli --version
   ```

2. Check extension settings:
   - `matter.lsp.enabled` should be `true`
   - `matter.lsp.path` empty (auto-discover `matter-lsp.exe`) or full path to **`matter-lsp.exe`** (not `matter-cli lsp`)

3. Restart VS Code

4. Check Output panel (View → Output → Matter Language Server)

### Commands Not Working

1. Verify Matter CLI is installed:
   ```bash
   matter-cli help
   ```

2. Check file extension is `.matter`

3. Restart VS Code

### Syntax Highlighting Not Working

1. Check file extension is `.matter`
2. Reload window (Ctrl+Shift+P → "Reload Window")
3. Reinstall extension

## Contributing

Contributions are welcome! Please visit:
https://github.com/matter-core/matter-core

## License

MIT License

## Links

- [Matter Core Repository](https://github.com/matter-core/matter-core)
- [Documentation](https://github.com/matter-core/matter-core/tree/main/docs)
- [Examples](https://github.com/matter-core/matter-core/tree/main/examples)
- [Report Issues](https://github.com/matter-core/matter-core/issues)

## Release Notes

### 0.7.0 (Initial Release)

- ✅ Syntax highlighting
- ✅ LSP integration (diagnostics, autocomplete, go-to-definition, hover, find references, rename)
- ✅ Code snippets
- ✅ Commands (run, compile, format, lint, debug)
- ✅ Formatter integration
- ✅ Linter integration
- ✅ File icons
- ✅ Theme support

---

**Enjoy coding with Matter!** 🚀
