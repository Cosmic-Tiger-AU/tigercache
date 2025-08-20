# Tiger Cache

Tiger Cache is an embedded fuzzy search library inspired by Typesense. It provides fast, typo-tolerant search over a local cache of documents, similar to how SQLite works for databases.

## Features

- **Embedded**: Self-contained library with no external dependencies
- **Fuzzy Search**: Tolerant of typos and misspellings
- **Trigram Indexing**: Fast candidate selection using trigram matching
- **Levenshtein Distance**: Precise similarity scoring
- **Persistence**: Save and load the index from a single file
- **Simple API**: Easy to integrate and use

## Core Concepts

1. **Inverted Index**: Maps search terms (tokens) to the documents that contain them
2. **Trigram Indexing**: Breaks words into smaller, overlapping chunks of three characters for fuzzy matching
3. **Levenshtein Distance**: Calculates the "edit distance" between search queries and potential matches
4. **Persistence**: Serializes the entire index to a single file

## Usage

```rust
use tiger_cache::{Document, TigerCache, SearchOptions};

// Create a new search engine
let mut tiger_cache = TigerCache::new();

// Add documents
let mut doc = Document::new("doc1");
doc.add_field("title", "Apple iPhone")
   .add_field("description", "The latest smartphone from Apple");
tiger_cache.add_document(doc).unwrap();

// Search with default options
let results = tiger_cache.search("iphone", None).unwrap();
for result in results {
    println!("Found: {} (score: {})", result.document.id, result.score);
}

// Search with custom options
let options = SearchOptions {
    max_distance: 2,        // Maximum Levenshtein distance
    score_threshold: 0.5,   // Minimum score threshold
    limit: 10,              // Maximum number of results
};
let results = tiger_cache.search("aple", Some(options)).unwrap(); // Typo: "aple" instead of "apple"

// Save to file
tiger_cache.save_to_file("search_index.bin").unwrap();

// Load from file
let tiger_cache = TigerCache::open("search_index.bin").unwrap();
```

## Example

Run the included example:

```bash
cargo run --example simple_search
```

This will start an interactive search demo with sample product data.

## How It Works

1. **Document Indexing**:
   - Documents are added to the index
   - Text fields are tokenized into words
   - Each token is broken into trigrams
   - Trigrams are mapped to tokens, and tokens are mapped to documents

2. **Search Process**:
   - The search query is tokenized and converted to trigrams
   - Candidate tokens are found by matching trigrams
   - Levenshtein distance is calculated between query tokens and candidates
   - Documents containing the best matching tokens are retrieved and scored
   - Results are sorted by relevance score

3. **Persistence**:
   - The entire index is serialized using bincode
   - The serialized data is written to a single file
   - The index can be loaded from the file later

## Performance Considerations

- The library keeps the entire index in memory for fast searching
- For very large datasets, consider sharding or using a more specialized solution
- The trigram approach provides a good balance between speed and accuracy for fuzzy search

## License

This project is licensed under the MIT License - see the LICENSE file for details.

