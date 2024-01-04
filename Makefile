.PHONY: test test-all clean clean-test clean-pyc clean-build docs help dist
.DEFAULT_GOAL := help

help:
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Dev

install: ## install the package in dev (editable) mode
	poetry install

lock: ## lock poetry dependencies
	poetry lock

test: ## run unit tests
	poetry run py.test

test-all: ## run tests on every Python version with tox
	tox

lint: ## check style
	poetry run flake8 lazynet tests

coverage: ## check code coverage
	poetry run coverage run --source lazynet -m pytest
	poetry run coverage report -m

clean: clean-build clean-pyc clean-test ## remove all build, test, coverage and Python artifacts

clean-all: clean
	poetry --rm

clean-build: ## remove build artifacts
	@rm -fr build/
	@rm -fr dist/
	@rm -fr .eggs/
	@find . -name '*.egg-info' -exec rm -fr {} +
	@find . -name '*.egg' -exec rm -f {} +

clean-pyc: ## remove Python file artifacts
	@find . -name '*.pyc' -exec rm -f {} +
	@find . -name '*.pyo' -exec rm -f {} +
	@find . -name '*~' -exec rm -f {} +
	@find . -name '__pycache__' -exec rm -fr {} +

clean-test: ## remove test and coverage artifacts
	@rm -fr .tox/
	@rm -f .coverage
	@rm -fr htmlcov/
	@rm -fr .pytest_cache

##@ Distribute

bump-major:
	bump2version major

bump-minor:
	bump2version minor

bump-patch:
	bump2version patch

dist: ## build the source and wheel packages
	poetry run python setup.py sdist
	poetry run python setup.py bdist_wheel
	ls -l dist
