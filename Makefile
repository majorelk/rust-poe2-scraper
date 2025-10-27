.PHONY: help fmt lint test run build clean check

# Default target
help:
	@echo "Available targets:"
	@echo "  make fmt     - Format code with rustfmt"
	@echo "  make lint    - Run clippy linter"
	@echo "  make test    - Run tests"
	@echo "  make run     - Run the application"
	@echo "  make build   - Build the project"
	@echo "  make check   - Run fmt, lint, and test"
	@echo "  make clean   - Clean build artifacts"

# Format code
fmt:
	cargo fmt

# Run clippy with warnings as errors
lint:
	cargo clippy -- -D warnings

# Run tests
test:
	cargo test

# Run the application with default args
run:
	cargo run

# Build the project
build:
	cargo build

# Run all checks (fmt, lint, test)
check: fmt lint test
	@echo "All checks passed!"

# Clean build artifacts
clean:
	cargo clean
