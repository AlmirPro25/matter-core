# Matter Core - WebAssembly Playground

Run Matter Core directly in your web browser using WebAssembly!

## Features

- ✅ **Full Matter Core Runtime** - Complete language support in the browser
- ✅ **Interactive Playground** - Write and execute code instantly
- ✅ **Bytecode Compilation** - View compiled bytecode
- ✅ **Example Library** - Learn from built-in examples
- ✅ **Zero Installation** - No setup required, just open in browser

## Building

### Prerequisites
- Rust toolchain
- wasm-pack (`cargo install wasm-pack`)

### Build WASM Module
```bash
cd crates/matter-wasm
wasm-pack build --target web --release
```

### Copy to Examples
```bash
cp -r pkg ../../examples/wasm/
```

## Running

### Local Development
```bash
cd examples/wasm
python -m http.server 8000
# Or use any static file server
```

Then open `http://localhost:8000` in your browser.

### Production
Deploy the `examples/wasm` directory to any static hosting service:
- GitHub Pages
- Netlify
- Vercel
- AWS S3
- etc.

## Usage

### Interactive Playground

1. **Write Code** - Enter Matter code in the left panel
2. **Run** - Click "▶️ Run" to execute
3. **Compile** - Click "🔧 Compile" to view bytecode
4. **Examples** - Click "📚 Examples" to load sample code
5. **Clear** - Click "🗑️ Clear" to reset output

### Programmatic Usage

```html
<script type="module">
    import init, { execute_matter, compile_matter } from './pkg/matter_wasm.js';

    await init();

    // Execute Matter code
    const result = execute_matter('let x = 42\nprint x');
    console.log(result);

    // Compile to bytecode
    const bytecode = compile_matter('let x = 42');
    console.log(bytecode);
</script>
```

### API Reference

#### `MatterWasm` Class

```javascript
// Create runtime
const runtime = new MatterWasm();

// Execute code
const result = runtime.execute(source);
// Returns: { success: bool, output: string, error: string? }

// Compile code
const bytecode = runtime.compile(source);
// Returns: { success: bool, output: string, error: string? }

// Get output buffer
const output = runtime.get_output();

// Clear output
runtime.clear_output();

// Get version
const version = MatterWasm.version();
```

#### Standalone Functions

```javascript
// Execute code (creates new runtime)
const result = execute_matter(source);

// Compile code (creates new runtime)
const bytecode = compile_matter(source);
```

## Examples

### Hello World
```matter
let message = "Hello, Matter!"
print message
```

### Fibonacci
```matter
fn fibonacci(n) {
    if n <= 1 {
        return n
    }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

print fibonacci(10)
```

### Loops
```matter
let sum = 0
let i = 1

while i <= 10 {
    set sum = sum + i
    set i = i + 1
}

print sum
```

## Performance

### WASM vs Native
- **Compilation:** ~2-5x slower than native
- **Execution:** ~1.5-3x slower than native
- **Startup:** ~50-100ms initial load
- **Memory:** ~2-5MB WASM module

### Optimization Tips
1. Use `--release` build for production
2. Enable LTO in Cargo.toml
3. Use `opt-level = "z"` for smaller binary
4. Compress WASM with gzip/brotli

## Browser Support

- ✅ Chrome 57+
- ✅ Firefox 52+
- ✅ Safari 11+
- ✅ Edge 16+

## Limitations

### Current Limitations
- ⚠️ No file system access
- ⚠️ No network requests (use fetch API from JS)
- ⚠️ Limited backend support (no native backends)
- ⚠️ No multi-threading (single-threaded WASM)

### Workarounds
- Use JavaScript interop for I/O
- Implement backends in JavaScript
- Use Web Workers for concurrency

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Browser Environment                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  JavaScript                                                 │
│      ↓                                                      │
│  wasm-bindgen (JS ↔ WASM bridge)                           │
│      ↓                                                      │
│  Matter WASM Module                                         │
│      ├─ Parser                                              │
│      ├─ Compiler                                            │
│      ├─ VM                                                  │
│      └─ Runtime                                             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Development

### Testing
```bash
cd crates/matter-wasm
wasm-pack test --headless --firefox
```

### Debugging
```javascript
// Enable debug mode
localStorage.setItem('matter_debug', 'true');

// View WASM memory
console.log(runtime.memory);

// Inspect bytecode
const bytecode = runtime.compile(source);
console.log(JSON.parse(bytecode.output));
```

## Deployment

### GitHub Pages
```bash
# Build WASM
cd crates/matter-wasm
wasm-pack build --target web --release

# Copy to docs folder
mkdir -p ../../docs/playground
cp -r pkg ../../docs/playground/
cp ../../examples/wasm/index.html ../../docs/playground/

# Commit and push
git add docs/playground
git commit -m "Deploy Matter WASM playground"
git push
```

### Netlify
```toml
# netlify.toml
[build]
  command = "cd crates/matter-wasm && wasm-pack build --target web --release && cp -r pkg ../../examples/wasm/"
  publish = "examples/wasm"
```

## Contributing

To add new features to the WASM module:

1. Implement in `crates/matter-wasm/src/lib.rs`
2. Add `#[wasm_bindgen]` attribute
3. Rebuild with `wasm-pack build`
4. Update JavaScript code in `index.html`
5. Test in browser
6. Document in this README

## Resources

- [wasm-bindgen Documentation](https://rustwasm.github.io/wasm-bindgen/)
- [WebAssembly MDN](https://developer.mozilla.org/en-US/docs/WebAssembly)
- [Rust and WebAssembly Book](https://rustwasm.github.io/docs/book/)

---

**Matter Core v0.8.0 - Now running in your browser!** 🚀
