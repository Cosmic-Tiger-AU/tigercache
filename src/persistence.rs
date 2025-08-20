use crate::error::Result;
use crate::index::Index;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

/// Save an index to a file
pub fn save_to_file<P: AsRef<Path>>(index: &Index, path: P) -> Result<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer(writer, index)?;
    Ok(())
}

/// Load an index from a file
pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Index> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let index: Index = serde_json::from_reader(reader)?;
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_index.bin");
        
        // Create a test index
        let mut index = Index::new();
        
        let mut doc1 = Document::new("doc1");
        doc1.add_field("title", "Test Document 1")
            .add_field("content", "This is the first test document");
        index.add_document(doc1).unwrap();
        
        let mut doc2 = Document::new("doc2");
        doc2.add_field("title", "Test Document 2")
            .add_field("content", "This is the second test document");
        index.add_document(doc2).unwrap();
        
        // Save the index to a file
        save_to_file(&index, &file_path).unwrap();
        
        // Load the index from the file
        let loaded_index = load_from_file(&file_path).unwrap();
        
        // Verify the loaded index
        assert_eq!(loaded_index.document_count(), 2);
        assert!(loaded_index.get_document("doc1").is_some());
        assert!(loaded_index.get_document("doc2").is_some());
        
        // Verify document content
        let doc1 = loaded_index.get_document("doc1").unwrap();
        assert_eq!(doc1.get_text_field("title").unwrap(), "Test Document 1");
        
        let doc2 = loaded_index.get_document("doc2").unwrap();
        assert_eq!(doc2.get_text_field("title").unwrap(), "Test Document 2");
    }
    
    #[test]
    fn test_save_to_nonexistent_directory() {
        let dir = tempdir().unwrap();
        let nonexistent_dir = dir.path().join("nonexistent");
        let file_path = nonexistent_dir.join("test_index.bin");
        
        let index = Index::new();
        
        // Saving to a nonexistent directory should fail
        let result = save_to_file(&index, &file_path);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_load_nonexistent_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("nonexistent.bin");
        
        // Loading a nonexistent file should fail
        let result = load_from_file(&file_path);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_load_corrupted_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("corrupted.bin");
        
        // Create a corrupted file
        std::fs::write(&file_path, b"this is not a valid index file").unwrap();
        
        // Loading a corrupted file should fail
        let result = load_from_file(&file_path);
        assert!(result.is_err());
    }
}
