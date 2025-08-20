# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Tiger Cache is an embedded fuzzy search library for Rust inspired by Typesense. It provides fast, typo-tolerant search over local document collections using trigram indexing and Levenshtein distance scoring.

## Common Development Commands

### Building and Testing
- `cargo build` - Build the project
- `cargo test` - Run all tests (unit and integration)
- `cargo test --verbose` - Run tests with verbose output
- `cargo clippy -- -D warnings` - Run linting (fails on warnings)
- `cargo run --example simple_search` - Run the interactive search demo

### Examples and Testing
- `cargo run --example dataset_test` - Run dataset testing example
- `cargo test test_name` - Run specific test
- Integration tests are in `tests/integration_tests.rs`

### Benchmarks (if added)
- `cargo bench` - Run benchmarks using criterion

## Architecture Overview

The codebase follows a modular design with clear separation of concerns:

### Core Components

1. **TigerCache** (`src/tiger_cache.rs`) - Main API entry point
   - Provides simple interface for document management and search
   - Handles persistence operations (save/load from file)
   - Wraps the underlying Index with convenience methods

2. **Index** (`src/index.rs`) - Core indexing engine
   - Maintains three key data structures:
     - `documents`: HashMap of document ID to Document
     - `inverted_index`: Maps tokens to document IDs containing them
     - `trigram_index`: Maps trigrams to tokens for fuzzy matching
   - Handles document addition/removal and index maintenance

3. **Search** (`src/search.rs`) - Search implementation
   - Implements fuzzy search using trigram matching + Levenshtein distance
   - Supports configurable search options (max distance, score threshold, result limits)
   - Returns scored results sorted by relevance

4. **Document** (`src/document.rs`) - Document representation
   - Flexible key-value field storage supporting text and numeric fields
   - Methods for field access and document manipulation

5. **Trigram** (`src/trigram.rs`) - Text processing utilities
   - Text normalization and tokenization
   - Trigram generation for fuzzy matching
   - Token extraction from text fields

6. **Persistence** (`src/persistence.rs`) - Serialization layer
   - Binary serialization using bincode for compact storage
   - Save/load entire index to/from single file

7. **Error** (`src/error.rs`) - Error handling
   - Custom error types using thiserror
   - Proper error propagation throughout the library

### Search Algorithm

The search process follows this flow:
1. Query tokenization and trigram generation
2. Candidate token discovery via trigram matching
3. Levenshtein distance filtering of candidates
4. Document retrieval and relevance scoring
5. Result ranking and limit application

### Key Design Patterns

- **Builder Pattern**: Document creation with method chaining
- **Options Pattern**: SearchOptions for configurable search behavior
- **Error Propagation**: Consistent Result<T> returns with custom error types
- **Serialization**: Complete index persistence to single binary file

## Testing Strategy

The project has comprehensive test coverage including:
- Unit tests in each module (`#[cfg(test)]` blocks)
- Integration tests covering full workflows
- Edge case testing (empty queries, large datasets, error conditions)
- Persistence testing with temporary files
- Concurrent access testing

When adding new features, ensure you add corresponding tests and run the full test suite.