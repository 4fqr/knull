# Knull Makefile

.PHONY: all build test clean install fmt lint run help

# Default target
all: build

# Build the compiler
build:
	cd src && cargo build --release --no-default-features

# Build debug version
build-debug:
	cd src && cargo build --no-default-features

# Run tests
test:
	cd src && cargo test

# Clean build artifacts
clean:
	cd src && cargo clean

# Install binary
install: build
	cp src/target/release/knull /usr/local/bin/knull

# Format code
fmt:
	cd src && cargo fmt

# Run linter
lint:
	cd src && cargo clippy

# Run a test file
run: build
	@echo "Usage: make run FILE=hello.knull"
	@if [ -n "$(FILE)" ]; then \
		./src/target/release/knull run $(FILE); \
	fi

# Quick test
check: build
	@echo "Usage: make check FILE=hello.knull"
	@if [ -n "$(FILE)" ]; then \
		./src/target/release/knull check $(FILE); \
	fi

# Show version
version:
	./src/target/release/knull --version

# Show help
help:
	@echo "Knull Build System"
	@echo ""
	@echo "Targets:"
	@echo "  build        Build release binary (default)"
	@echo "  build-debug  Build debug binary"
	@echo "  test         Run tests"
	@echo "  clean        Clean build artifacts"
	@echo "  install      Install to /usr/local/bin"
	@echo "  fmt          Format code"
	@echo "  lint         Run linter"
	@echo "  run          Run a .knull file (FILE=...)"
	@echo "  check        Check syntax (FILE=...)"
	@echo "  version      Show version"
	@echo "  help         Show this help"
	@echo ""
	@echo "Examples:"
	@echo "  make build"
	@echo "  make run FILE=examples/hello_world.knull"
	@echo "  make install"
