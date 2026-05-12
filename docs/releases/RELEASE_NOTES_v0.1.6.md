# Matter Core v0.1.6 - Visual Backend Integration

**Release Date:** May 9, 2026  
**Status:** ✅ Stable Release  
**Tests:** 28/28 passing (100%)

---

## 🎨 Major Feature: Visual Backend (PVM/PXL Integration)

Matter Core now includes **official visual backend integration** with PVM/PXL, maintaining complete architectural decoupling.

### What's New

#### New Crate: `matter-visual`

A dedicated crate for visual system integration:
- `VisualRuntime` trait (contract for PVM)
- `TraceVisualBackend` (mock/trace implementation)
- `PvmVisualBackend` (placeholder for future real integration)
- Complete visual API with 6 commands

#### Visual API

```matter
# Execute visual application
visual.run("pizzaria")

# Load visual bytecode (PVMBC)
visual.load("apps/pizzaria.pvmbc")

# Create visual surface
visual.surface("main", 1080, 1920)

# Create visual region
visual.region("checkout", 100, 200, 300, 80)

# Animate region
visual.pulse("checkout")

# Set region property
visual.set("checkout", "energy", 80)
```

#### Event Integration

Visual commands work seamlessly with Matter events:

```matter
on boot {
    visual.surface("main", 1080, 1920)
    visual.region("button", 100, 100, 200, 50)
}

on tap {
    visual.pulse("button")
}
```

---

## 📦 What's Included

### New Examples (4)

1. **`examples/visual_basic.matter`**
   - Basic visual commands
   - Surface, region, pulse, run

2. **`examples/visual_event.matter`**
   - Event integration
   - Boot and tap handlers

3. **`examples/visual_advanced.matter`**
   - Visual properties
   - Energy, material, behavior

4. **`examples/visual_load.matter`**
   - PVMBC loading
   - Pre-compiled visual apps

### New Tests (12)

- 6 unit tests in `matter-visual`
- 6 integration tests in `visual_backend_test`
- All tests passing (100%)

### New Documentation (5)

1. **`docs/VISUAL_BACKEND.md`** (500+ lines)
   - Complete visual backend guide
   - API reference
   - Architecture and principles

2. **`QUICKSTART_VISUAL.md`**
   - Quick start guide
   - Practical examples
   - Troubleshooting

3. **`VISUAL_INTEGRATION_SUMMARY.md`**
   - Executive summary
   - Implementation details

4. **`VISUAL_BACKEND_COMPLETE.md`**
   - Complete checklist
   - Success criteria

5. **`INTEGRATION_SUCCESS.md`**
   - Success summary
   - Statistics

### Updated Documentation (3)

- `README.md` - Visual backend section
- `docs/SPEC.md` - Visual API specification
- `docs/ARCHITECTURE.md` - Visual backend architecture

---

## 🏗️ Architecture

### Decoupled Design

Matter Core remains a **general-purpose language**. Visual is a **pluggable backend**.

```
Matter Core (general language)
    ↓
Backends (pluggable)
    ├── agent (AI/LLM)
    ├── visual (PVM/PXL) ← NEW
    ├── store (persistence)
    └── net (network)
```

### Principles

1. ✅ **Decoupling**: Matter does NOT depend on PVM
2. ✅ **Contract first**: API defined before implementation
3. ✅ **Testability**: Mock allows testing without PVM
4. ✅ **Independent evolution**: Matter and PVM grow separately

---

## 📊 Statistics

### Before v0.1.6

- Crates: 9
- Tests: 22
- Examples: 18
- Backends: 3

### After v0.1.6

- **Crates: 10** (+1)
- **Tests: 28** (+6)
- **Examples: 22** (+4)
- **Backends: 4** (+1)
- **Tests passing: 100%** ✅

---

## 🚀 Getting Started

### Installation

```bash
# Clone repository
git clone <repo-url>
cd matter-core

# Build
cargo build --release

# Or use installer (Windows)
.\install.ps1
```

### Try Visual Backend

```bash
# Run basic example
matter run examples/visual_basic.matter

# Run with events
matter emit examples/visual_event.matter boot

# Run advanced example
matter emit examples/visual_advanced.matter boot
```

### Expected Output

```
[VISUAL] surface main 1080x1920
[VISUAL] region checkout x=100 y=200 w=300 h=80
[VISUAL] pulse checkout
[VISUAL] run pizzaria
```

---

## 🧪 Testing

### Run All Tests

```bash
cargo test
```

**Result:** 28/28 tests passing (100%)

### Run Visual Tests Only

```bash
# Unit tests
cargo test --package matter-visual

# Integration tests
cargo test --test visual_backend_test
```

---

## 📚 Documentation

### Quick Start

- **`QUICKSTART_VISUAL.md`** - Get started with visual backend

### Complete Guide

- **`docs/VISUAL_BACKEND.md`** - Complete documentation

### API Reference

- **`docs/SPEC.md`** - Language specification with visual API

### Architecture

- **`docs/ARCHITECTURE.md`** - Technical architecture

---

## 🔄 Migration Guide

### From v0.1.5 to v0.1.6

**No breaking changes!** This is an additive release.

#### New Features Available

```matter
# You can now use visual commands
visual.surface("main", 1080, 1920)
visual.region("button", 100, 100, 200, 50)
visual.pulse("button")
```

#### Existing Code

All existing code continues to work without changes.

---

## 🐛 Bug Fixes

None in this release (additive only).

---

## ⚠️ Known Limitations

### Current Implementation

The current implementation uses `TraceVisualBackend`, which:
- ✅ Prints visual commands to console
- ✅ Validates API usage
- ✅ Allows testing without PVM
- ❌ Does not render actual visuals (yet)

### Future Integration

When PVM is ready:
1. Implement `PvmVisualBackend`
2. Connect with PVM runtime
3. Load PVMBC
4. Render SmartPixels
5. Bidirectional events

---

## 🎯 Roadmap

### v0.2 (Next)

- [ ] Data Model enhancements
- [ ] Standard Library expansion
- [ ] REPL improvements
- [ ] Error system refinements

### v0.3 (Future)

- [ ] PvmVisualBackend real implementation
- [ ] PXL compiler integration
- [ ] Visual debugging tools
- [ ] Performance optimizations

---

## 🙏 Acknowledgments

This release maintains Matter Core's architectural principles:
- Contract before implementation
- Complete decoupling
- Testability from day one
- Comprehensive documentation

---

## 📞 Support

### Documentation

- `docs/VISUAL_BACKEND.md` - Complete guide
- `QUICKSTART_VISUAL.md` - Quick start
- `README.md` - Project overview

### Examples

- `examples/visual_*.matter` - 4 visual examples

### Testing

```bash
cargo test --package matter-visual
cargo test --test visual_backend_test
```

---

## 🎉 Conclusion

Matter Core v0.1.6 brings **official visual backend integration** while maintaining the clean, decoupled architecture that defines the project.

**Matter is not just a language. It's a runtime-oriented language system with pluggable backends.**

And now, **visual is one of those backends**. 🎨✨

---

**Download:** [Release v0.1.6](link-to-release)  
**Changelog:** See `PROGRESS.md` for complete history  
**Next Release:** v0.2 (Data Model enhancements)

