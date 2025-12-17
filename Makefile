.PHONY: help install dev test lint coverage bench bench-rust bench-py clean clean-all clean-build clean-pyc clean-test dist publish
.DEFAULT_GOAL := help

help:
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Dev

install: ## Sync dependencies and build extension
	uv sync --group dev
	uv run maturin develop

dev: ## Build in release mode
	uv run maturin develop --release

test: ## Run tests
	uv run pytest

lint: ## Check style
	uv run ruff check lazynet tests
	cargo clippy

coverage: ## Check code coverage
	uv run coverage run --source lazynet -m pytest
	uv run coverage report -m

##@ Benchmarks (requires server on localhost:8080)

bench: bench-rust bench-py ## Run all benchmarks

bench-rust: ## Run Rust benchmarks
	cargo run --bin bench_runner --release

bench-py: ## Run Python benchmarks (pytest-benchmark)
	uv run pytest tests/test_perf.py --benchmark-only -v

clean: clean-build clean-pyc clean-test ## Remove all build, test, coverage and Python artifacts

clean-all: clean ## Also remove Rust target directory and venv
	@rm -rf target/
	@rm -rf .venv/

clean-build: ## Remove build artifacts
	@rm -rf target/
	@rm -rf dist/
	@rm -rf *.egg-info
	@find . -name '*.so' -exec rm -f {} +

clean-pyc: ## Remove Python file artifacts
	@find . -name '*.pyc' -exec rm -f {} +
	@find . -name '*.pyo' -exec rm -f {} +
	@find . -name '*~' -exec rm -f {} +
	@find . -name '__pycache__' -exec rm -fr {} +

clean-test: ## Remove test and coverage artifacts
	@rm -rf .tox/
	@rm -f .coverage
	@rm -rf htmlcov/
	@rm -rf .pytest_cache

##@ Distribute

dist: ## Build wheels
	uv run maturin build --release

publish: ## Publish to PyPI
	uv run maturin publish
