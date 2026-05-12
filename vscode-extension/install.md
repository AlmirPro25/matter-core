# Installing Matter VS Code Extension

## Prerequisites

1. **VS Code** installed (version 1.75.0 or higher)
2. **Matter CLI** installed and in PATH
   ```bash
   matter-cli --version
   ```

## Installation Methods

### Method 1: From Source (Development)

1. **Navigate to extension directory:**
   ```bash
   cd vscode-extension
   ```

2. **Install dependencies:**
   ```bash
   npm install
   ```

3. **Open in VS Code:**
   ```bash
   code .
   ```

4. **Press F5** to open Extension Development Host

5. **Test the extension** in the new window

### Method 2: Package and Install

1. **Install vsce (VS Code Extension Manager):**
   ```bash
   npm install -g @vscode/vsce
   ```

2. **Navigate to extension directory:**
   ```bash
   cd vscode-extension
   ```

3. **Install dependencies:**
   ```bash
   npm install
   ```

4. **Package the extension:**
   ```bash
   vsce package
   ```
   This creates `matter-0.7.0.vsix`

5. **Install the VSIX:**
   ```bash
   code --install-extension matter-0.7.0.vsix
   ```

6. **Restart VS Code**

### Method 3: Manual Installation

1. **Package the extension** (see Method 2, steps 1-4)

2. **Open VS Code**

3. **Go to Extensions** (Ctrl+Shift+X)

4. **Click "..." menu** (top right)

5. **Select "Install from VSIX..."**

6. **Choose** `matter-0.7.0.vsix`

7. **Restart VS Code**

## Verification

1. **Open a .matter file** or create a new one

2. **Check syntax highlighting** - keywords should be colored

3. **Test autocomplete** - type `agent.` and press Ctrl+Space

4. **Run a file:**
   - Press Ctrl+Shift+P
   - Type "Matter: Run File"
   - Should execute the file

5. **Check LSP:**
   - Create a syntax error
   - Should see red squiggly line
   - Hover to see error message

## Configuration

After installation, configure the extension:

1. **Open Settings** (Ctrl+,)

2. **Search for "Matter"**

3. **Configure:**
   - `matter.lsp.enabled`: Enable/disable LSP (default: true)
   - `matter.lsp.path`: Path to matter-cli (default: "matter-cli")
   - `matter.formatter.enabled`: Enable formatter (default: true)
   - `matter.linter.enabled`: Enable linter (default: true)

## Troubleshooting

### Extension Not Loading

1. Check VS Code version:
   ```bash
   code --version
   ```
   Should be 1.75.0 or higher

2. Check extension is installed:
   - Open Extensions (Ctrl+Shift+X)
   - Search for "Matter"
   - Should appear in list

3. Reload window:
   - Press Ctrl+Shift+P
   - Type "Reload Window"

### LSP Not Working

1. Verify matter-cli is in PATH:
   ```bash
   matter-cli --version
   ```

2. Check LSP is enabled:
   - Settings → Matter → LSP Enabled (should be checked)

3. Check Output panel:
   - View → Output
   - Select "Matter Language Server" from dropdown
   - Look for errors

4. Restart LSP:
   - Press Ctrl+Shift+P
   - Type "Reload Window"

### Syntax Highlighting Not Working

1. Check file extension is `.matter`

2. Check language mode:
   - Bottom right of VS Code
   - Should say "Matter"
   - If not, click and select "Matter"

3. Reload window:
   - Press Ctrl+Shift+P
   - Type "Reload Window"

### Commands Not Appearing

1. Check file is open and has `.matter` extension

2. Press Ctrl+Shift+P and type "Matter"
   - Should see all Matter commands

3. If not appearing:
   - Reload window
   - Reinstall extension

## Uninstallation

1. **Open Extensions** (Ctrl+Shift+X)

2. **Find "Matter Language"**

3. **Click gear icon** → **Uninstall**

4. **Restart VS Code**

## Next Steps

After installation:

1. **Open example files:**
   ```bash
   cd examples
   code hello.matter
   ```

2. **Try commands:**
   - Matter: Run File
   - Matter: Format File
   - Matter: Show Backends

3. **Explore snippets:**
   - Type `fn` and press Tab
   - Type `if` and press Tab
   - Type `agent.say` and press Tab

4. **Read documentation:**
   - See README.md in extension folder
   - See docs/ in Matter Core repository

## Support

- **Issues:** https://github.com/matter-core/matter-core/issues
- **Documentation:** https://github.com/matter-core/matter-core/tree/main/docs
- **Examples:** https://github.com/matter-core/matter-core/tree/main/examples

---

**Enjoy coding with Matter!** 🚀
