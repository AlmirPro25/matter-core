# Matter VS Code Extension - Quick Start

## 🚀 5-Minute Setup

### Step 1: Install Matter CLI

```bash
# Windows
cd "F:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"
.\install.ps1

# Verify installation
matter-cli --version
```

### Step 2: Install VS Code Extension

**Option A: From VSIX (Recommended)**
```bash
cd vscode-extension
npm install
npm install -g @vscode/vsce
vsce package
code --install-extension matter-0.7.0.vsix
```

**Option B: Development Mode**
```bash
cd vscode-extension
npm install
code .
# Press F5 to open Extension Development Host
```

### Step 3: Restart VS Code

Close and reopen VS Code to activate the extension.

### Step 4: Test the Extension

1. **Create a test file:**
   ```bash
   code test.matter
   ```

2. **Write some Matter code:**
   ```matter
   let x = 10
   print x
   
   fn dobro(n) {
       return n * 2
   }
   
   print dobro(21)
   ```

3. **See syntax highlighting** - Keywords should be colored

4. **Test autocomplete:**
   - Type `agent.` and press `Ctrl+Space`
   - Should see: `say`, `think`, `learn`

5. **Run the file:**
   - Press `Ctrl+Shift+P`
   - Type "Matter: Run File"
   - Should execute and show output

## ✨ Key Features

### 1. Syntax Highlighting
- Keywords are colored
- Backend calls are highlighted
- Strings, numbers, comments have distinct colors

### 2. Autocomplete (Ctrl+Space)
```matter
# Type and press Ctrl+Space:
agent.|     # Shows: say, think, learn
visual.|    # Shows: run, load, surface, region, pulse, set
store.|     # Shows: get, set, delete
```

### 3. Go-to-Definition (F12)
```matter
fn soma(a, b) { return a + b }
let result = soma(10, 20)  # F12 on 'soma' goes to definition
```

### 4. Hover Information
```matter
fn soma(a, b) { return a + b }
let result = soma(10, 20)  # Hover over 'soma' to see signature
```

### 5. Find References (Shift+F12)
```matter
let x = 10
print x  # Shift+F12 on 'x' shows all uses
set x = 20
```

### 6. Rename Symbol (F2)
```matter
let x = 10
print x  # F2 on 'x' to rename everywhere
set x = 20
```

### 7. Commands (Ctrl+Shift+P)
- **Matter: Run File** - Execute current file
- **Matter: Compile File** - Compile to bytecode
- **Matter: Format File** - Format code
- **Matter: Lint File** - Analyze code
- **Matter: Debug File** - Start debugger

### 8. Snippets (Type + Tab)
- `fn` + Tab → Function declaration
- `if` + Tab → If statement
- `while` + Tab → While loop
- `for` + Tab → For loop
- `on` + Tab → Event handler

### 9. Format Document (Shift+Alt+F)
```matter
# Before:
let x=10
fn soma(a,b){return a+b}

# After (Shift+Alt+F):
let x = 10
fn soma(a, b) {
    return a + b
}
```

### 10. Real-time Errors
```matter
print y  # Red squiggly line: undefined variable 'y'
```

## 🎯 Common Workflows

### Workflow 1: Write and Run
1. Create `.matter` file
2. Write code (with autocomplete)
3. Press `Ctrl+Shift+P` → "Matter: Run File"
4. See output in Output panel

### Workflow 2: Debug
1. Open `.matter` file
2. Press `Ctrl+Shift+P` → "Matter: Debug File"
3. Use debugger commands:
   - `break 10` - Set breakpoint at line 10
   - `continue` - Continue execution
   - `step` - Step into
   - `locals` - Show local variables

### Workflow 3: Format and Lint
1. Write code
2. Press `Shift+Alt+F` to format
3. Press `Ctrl+Shift+P` → "Matter: Lint File"
4. Fix any issues reported

### Workflow 4: Navigate Code
1. `F12` - Go to definition
2. `Shift+F12` - Find references
3. `F2` - Rename symbol
4. `Ctrl+Shift+O` - Go to symbol in file

## 🔧 Configuration

### Open Settings (Ctrl+,)

Search for "Matter" and configure:

```json
{
  // Enable LSP (default: true)
  "matter.lsp.enabled": true,
  "matter.lsp.path": "",
  "matter.cli.path": "matter-cli",

  // Enable formatter (default: true)
  "matter.formatter.enabled": true,
  
  // Enable linter (default: true)
  "matter.linter.enabled": true,
  
  // Format on save
  "editor.formatOnSave": true,
  
  // Auto-save
  "files.autoSave": "afterDelay"
}
```

## 🐛 Troubleshooting

### Extension Not Working

1. **Check Matter CLI is installed:**
   ```bash
   matter-cli --version
   ```

2. **Check extension is active:**
   - Open Extensions (Ctrl+Shift+X)
   - Search "Matter"
   - Should show "Matter Language" as installed

3. **Reload window:**
   - Press `Ctrl+Shift+P`
   - Type "Reload Window"

### LSP Not Connecting

1. **Check Output panel:**
   - View → Output
   - Select "Matter Language Server"
   - Look for errors

2. **Check settings:**
   - Settings → Matter → LSP Enabled (should be checked)

3. **Restart LSP:**
   - Reload window (Ctrl+Shift+P → "Reload Window")

### Syntax Highlighting Not Working

1. **Check file extension is `.matter`**

2. **Check language mode:**
   - Bottom right corner should say "Matter"
   - If not, click and select "Matter"

3. **Reload window**

## 📚 Examples

### Example 1: Hello World
```matter
print "Hello, Matter!"
```

### Example 2: Function
```matter
fn soma(a, b) {
    return a + b
}

print soma(10, 20)  # 30
```

### Example 3: Event Handler
```matter
on boot {
    print "System started"
    agent.say("Hello from Matter!")
}
```

### Example 4: Backend Integration
```matter
# Store data
store.set("counter", 0)

# Get data
let count = store.get("counter")
set count = count + 1
store.set("counter", count)

print count
```

### Example 5: Lists and Loops
```matter
let numbers = [1, 2, 3, 4, 5]

for num in numbers {
    print num * 2
}
```

## 🎓 Learning Resources

### Documentation
- `docs/GETTING_STARTED.md` - Getting started guide
- `docs/TUTORIAL.md` - Complete tutorial
- `docs/SPEC.md` - Language specification
- `docs/ARCHITECTURE.md` - System architecture

### Examples
- `examples/hello.matter` - Hello world
- `examples/functions.matter` - Functions
- `examples/loops.matter` - Loops
- `examples/events.matter` - Events
- `examples/backends.matter` - Backend integration
- `examples/apps/` - Complete applications

### Commands
```bash
# Show available backends
matter-cli backends

# Show examples
matter-cli examples

# Run example
matter-cli run examples/hello.matter

# REPL
matter-cli repl
```

## 🚀 Next Steps

1. **Explore examples:**
   ```bash
   cd examples
   code .
   ```

2. **Try REPL:**
   ```bash
   matter-cli repl
   ```

3. **Build something:**
   - Counter app
   - Todo list
   - API client
   - Data analyzer

4. **Read documentation:**
   - `docs/GETTING_STARTED.md`
   - `docs/TUTORIAL.md`

5. **Join community:**
   - GitHub: https://github.com/matter-core/matter-core
   - Issues: https://github.com/matter-core/matter-core/issues

## 💡 Tips

1. **Use autocomplete** - Press `Ctrl+Space` frequently
2. **Use snippets** - Type `fn`, `if`, `for` and press Tab
3. **Format often** - Press `Shift+Alt+F` to keep code clean
4. **Check errors** - Red squiggly lines show problems
5. **Navigate with F12** - Jump to definitions quickly
6. **Use debugger** - Set breakpoints and inspect variables
7. **Read hover info** - Hover over symbols for information

## 🎉 You're Ready!

You now have a complete Matter Core development environment with:
- ✅ Syntax highlighting
- ✅ Autocomplete
- ✅ Error detection
- ✅ Code navigation
- ✅ Debugging
- ✅ Formatting
- ✅ Linting

**Happy coding with Matter!** 🚀

---

**Need help?**
- Documentation: `docs/`
- Examples: `examples/`
- Issues: https://github.com/matter-core/matter-core/issues
