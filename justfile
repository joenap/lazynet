
default:
	@just --list --unsorted | lolcat

# Dev

# Sync dependencies and build extension
install:
	uv sync --group dev
	uv run maturin develop

# Build in release mode
dev:
	uv run maturin develop --release

# Run tests
test:
	uv run pytest

# Check style
lint:
	uv run ruff check lazynet tests
	cargo clippy

# Check code coverage
coverage:
	uv run coverage run --source lazynet -m pytest
	uv run coverage report -m

# Benchmarks (requires server on localhost:8080)

# Run all benchmarks
bench: bench-rust bench-py

# Run Rust benchmarks
bench-rust:
	cargo run --bin bench_runner --release

# Run Python benchmarks (pytest-benchmark)
bench-py:
	uv run pytest tests/test_perf.py --benchmark-only -v

# Remove all build, test, coverage and Python artifacts
clean: clean-build clean-pyc clean-test

# Also remove Rust target directory and venv
clean-all: clean
	rm -rf target/
	rm -rf .venv/

# Remove build artifacts
clean-build:
	rm -rf target/
	rm -rf dist/
	rm -rf *.egg-info
	find . -name '*.so' -exec rm -f {} +

# Remove Python file artifacts
clean-pyc:
	find . -name '*.pyc' -exec rm -f {} +
	find . -name '*.pyo' -exec rm -f {} +
	find . -name '*~' -exec rm -f {} +
	find . -name '__pycache__' -exec rm -fr {} +

# Remove test and coverage artifacts
clean-test:
	rm -rf .tox/
	rm -f .coverage
	rm -rf htmlcov/
	rm -rf .pytest_cache

# Release

# Bump version (e.g., just bump 0.9.0)
bump version:
	sed -i '' 's/^version = ".*"/version = "{{version}}"/' Cargo.toml pyproject.toml
	git add Cargo.toml pyproject.toml
	git commit -m "Bump version to {{version}}"
	git tag v{{version}}

# Distribute

# Build wheels
dist:
	uv run maturin build --release

# Publish to PyPI
publish:
	uv run maturin publish
