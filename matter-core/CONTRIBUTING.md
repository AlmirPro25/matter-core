# Contributing to Matter

Thank you for your interest in contributing to Matter! 🎉

## 🌟 **Ways to Contribute**

1. **Code Contributions** - Bug fixes, features, optimizations
2. **Documentation** - Tutorials, examples, guides
3. **Bug Reports** - Help us find and fix issues
4. **Feature Requests** - Suggest new features
5. **Community** - Help others on Discord/GitHub

## 🚀 **Getting Started**

### **1. Fork and Clone**
```bash
git clone https://github.com/YOUR_USERNAME/matter.git
cd matter
```

### **2. Build**
```bash
cargo build
cargo test
```

### **3. Make Changes**
```bash
git checkout -b my-feature
# Make your changes
git commit -m "Add my feature"
git push origin my-feature
```

### **4. Submit PR**
- Open a pull request on GitHub
- Describe your changes
- Link any related issues

## 📋 **Code Guidelines**

### **Rust Code:**
- Follow Rust style guide
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add tests for new features
- Document public APIs

### **Matter Code:**
- Follow Matter style guide
- Add examples for new features
- Update documentation

### **Commits:**
- Use clear, descriptive commit messages
- Reference issues: "Fix #123: Description"
- Keep commits focused and atomic

## 🧪 **Testing**

### **Run Tests:**
```bash
# All tests
cargo test

# Specific crate
cargo test -p matter-parser

# With output
cargo test -- --nocapture
```

### **Add Tests:**
- Unit tests in same file as code
- Integration tests in `tests/` directory
- Examples in `examples/` directory

## 📚 **Documentation**

### **Code Documentation:**
```rust
/// Brief description
///
/// # Examples
///
/// ```
/// let x = example();
/// ```
pub fn example() -> i32 {
    42
}
```

### **User Documentation:**
- Add tutorials to `docs/`
- Add examples to `examples/`
- Update README.md if needed

## 🐛 **Bug Reports**

### **Good Bug Report:**
```markdown
**Description:**
Clear description of the bug

**Steps to Reproduce:**
1. Step 1
2. Step 2
3. Step 3

**Expected Behavior:**
What should happen

**Actual Behavior:**
What actually happens

**Environment:**
- Matter version: v2.5.0
- OS: Windows 11
- Rust version: 1.75.0

**Additional Context:**
Any other relevant information
```

## 💡 **Feature Requests**

### **Good Feature Request:**
```markdown
**Problem:**
What problem does this solve?

**Proposed Solution:**
How should it work?

**Alternatives:**
What other solutions did you consider?

**Examples:**
Code examples of how it would be used

**Additional Context:**
Any other relevant information
```

## 🏆 **Recognition**

Contributors are recognized in:
- CONTRIBUTORS.md
- Release notes
- Documentation credits

## 📞 **Getting Help**

- **Discord:** https://discord.gg/matter
- **GitHub Discussions:** https://github.com/matter-lang/matter/discussions
- **Email:** support@matter-lang.org

## 📜 **Code of Conduct**

Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## 📄 **License**

By contributing, you agree that your contributions will be licensed under the MIT License.

---

**Thank you for contributing to Matter!** 🚀

Together, we're building the future of programming! 🌍
