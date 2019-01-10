.PHONY: test test-all clean clean-test clean-pyc clean-build docs help dist
.DEFAULT_GOAL := help

define PRINT_HELP_PYSCRIPT
import re, sys

for line in sys.stdin:
	match = re.match(r'^([a-zA-Z_-]+):.*?## (.*)$$', line)
	if match:
		target, help = match.groups()
		print("%-20s %s" % (target, help))
endef
export PRINT_HELP_PYSCRIPT

help:
	@python -c "$$PRINT_HELP_PYSCRIPT" < $(MAKEFILE_LIST)

bootstrap: ## bootstrap the environment
	pipenv --python 3.6
	# pipenv run pip install pip==18.0

install: ## install the package in dev (editable) mode
	pipenv run python setup.py develop
	pipenv run pip install -r requirements.txt

-: ## ---

test: ## run unit tests
	pipenv run py.test

test-all: ## run tests on every Python version with tox
	tox

lint: ## check style
	pipenv run flake8 httpstream tests

coverage: ## check code coverage
	pipenv run coverage run --source httpstream -m pytest
	pipenv run coverage report -m

-: ## ---

dist: ## build the source and wheel packages
	pipenv run python setup.py sdist
	pipenv run python setup.py bdist_wheel
	ls -l dist

-: ## ---

clean: clean-build clean-pyc clean-test ## remove all build, test, coverage and Python artifacts

clean-all: clean
	pipenv --rm

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
