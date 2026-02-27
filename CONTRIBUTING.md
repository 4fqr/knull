# Contributing to Knull

Thank you for your interest in contributing to Knull!

## Code of Conduct

We are committed to providing a welcoming and inclusive experience for everyone. Be excellent to each other.

## How to Contribute

### Reporting Bugs

1. Check if the bug has already been reported
2. Create a detailed issue with:
   - Clear title
   - Steps to reproduce
   - Expected vs actual behavior
   - Environment details

### Suggesting Features

1. Open an issue with `[Feature Request]` prefix
2. Describe the feature
3. Explain use cases
4. Show example syntax (if applicable)

### Pull Requests

1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feature/my-feature`
3. **Make** your changes
4. **Test** your changes
5. **Commit** with clear messages
6. **Push** to your fork
7. **Submit** a Pull Request

## Development Setup

```bash
# Clone the repository
git clone https://github.com/4fqr/knull.git
cd knull

# Build
cd src
cargo build --release --no-default-features

# Test
cargo test
```

## Coding Standards

- Follow the existing code style
- Use meaningful variable/function names
- Add comments for complex logic
- Keep functions small and focused

## Commit Messages

Use clear, descriptive commit messages:

```
feat: add port scanner example
fix: resolve lexer lifetime issue
docs: update installation guide
```

## Pull Request Guidelines

- Keep PRs focused and atomic
- Reference related issues
- Include tests if applicable
- Update documentation if needed

## Areas to Contribute

| Area | Priority |
|------|----------|
| Compiler (Lexer/Parser) | High |
| Code Generator | High |
| Standard Library | Medium |
| Examples | Medium |
| Documentation | Medium |
| Editor Support | Low |

## Communication

- GitHub Issues: Bug reports and feature requests
- Discussions: General questions

---

**Thank you for contributing to Knull!**
