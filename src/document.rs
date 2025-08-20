use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A document that can be indexed and searched
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier for the document
    pub id: String,
    
    /// Fields containing the document data
    pub fields: HashMap<String, serde_json::Value>,
}

impl Document {
    /// Create a new document with the given ID
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            fields: HashMap::new(),
        }
    }

    /// Add a field to the document
    pub fn add_field<T>(&mut self, name: impl Into<String>, value: T) -> &mut Self
    where
        T: Serialize,
    {
        if let Ok(value) = serde_json::to_value(value) {
            self.fields.insert(name.into(), value);
        }
        self
    }

    /// Get a field value as a string if it exists and is a string
    pub fn get_text_field(&self, name: &str) -> Option<String> {
        self.fields.get(name).and_then(|value| {
            if let serde_json::Value::String(s) = value {
                Some(s.clone())
            } else {
                // Convert non-string values to string representation
                Some(value.to_string())
            }
        })
    }

    /// Get all text fields as a vector of strings
    pub fn get_all_text_fields(&self) -> Vec<String> {
        self.fields
            .iter()
            .filter_map(|(_, value)| {
                if let serde_json::Value::String(s) = value {
                    Some(s.clone())
                } else if !value.is_object() && !value.is_array() {
                    // Convert simple non-string values to string representation
                    Some(value.to_string())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::new("test_id");
        assert_eq!(doc.id, "test_id");
        assert!(doc.fields.is_empty());
    }

    #[test]
    fn test_add_field_string() {
        let mut doc = Document::new("test_id");
        doc.add_field("title", "Test Title");
        
        assert_eq!(doc.fields.len(), 1);
        assert!(doc.fields.contains_key("title"));
        
        if let serde_json::Value::String(value) = &doc.fields["title"] {
            assert_eq!(value, "Test Title");
        } else {
            panic!("Field value is not a string");
        }
    }

    #[test]
    fn test_add_field_number() {
        let mut doc = Document::new("test_id");
        doc.add_field("count", 42);
        
        assert_eq!(doc.fields.len(), 1);
        assert!(doc.fields.contains_key("count"));
        
        if let serde_json::Value::Number(value) = &doc.fields["count"] {
            assert_eq!(value.as_i64().unwrap(), 42);
        } else {
            panic!("Field value is not a number");
        }
    }

    #[test]
    fn test_add_field_boolean() {
        let mut doc = Document::new("test_id");
        doc.add_field("active", true);
        
        assert_eq!(doc.fields.len(), 1);
        assert!(doc.fields.contains_key("active"));
        
        if let serde_json::Value::Bool(value) = &doc.fields["active"] {
            assert_eq!(*value, true);
        } else {
            panic!("Field value is not a boolean");
        }
    }

    #[test]
    fn test_add_multiple_fields() {
        let mut doc = Document::new("test_id");
        doc.add_field("title", "Test Title")
           .add_field("count", 42)
           .add_field("active", true);
        
        assert_eq!(doc.fields.len(), 3);
        assert!(doc.fields.contains_key("title"));
        assert!(doc.fields.contains_key("count"));
        assert!(doc.fields.contains_key("active"));
    }

    #[test]
    fn test_get_text_field_string() {
        let mut doc = Document::new("test_id");
        doc.add_field("title", "Test Title");
        
        let value = doc.get_text_field("title");
        assert_eq!(value, Some("Test Title".to_string()));
    }

    #[test]
    fn test_get_text_field_number() {
        let mut doc = Document::new("test_id");
        doc.add_field("count", 42);
        
        let value = doc.get_text_field("count");
        assert_eq!(value, Some("42".to_string()));
    }

    #[test]
    fn test_get_text_field_boolean() {
        let mut doc = Document::new("test_id");
        doc.add_field("active", true);
        
        let value = doc.get_text_field("active");
        assert_eq!(value, Some("true".to_string()));
    }

    #[test]
    fn test_get_text_field_nonexistent() {
        let doc = Document::new("test_id");
        let value = doc.get_text_field("nonexistent");
        assert_eq!(value, None);
    }

    #[test]
    fn test_get_all_text_fields() {
        let mut doc = Document::new("test_id");
        doc.add_field("title", "Test Title")
           .add_field("description", "Test Description")
           .add_field("count", 42)
           .add_field("active", true);
        
        let fields = doc.get_all_text_fields();
        assert_eq!(fields.len(), 4);
        assert!(fields.contains(&"Test Title".to_string()));
        assert!(fields.contains(&"Test Description".to_string()));
        assert!(fields.contains(&"42".to_string()));
        assert!(fields.contains(&"true".to_string()));
    }

    #[test]
    fn test_get_all_text_fields_with_complex_types() {
        let mut doc = Document::new("test_id");
        doc.add_field("title", "Test Title")
           .add_field("tags", vec!["tag1", "tag2"])
           .add_field("metadata", serde_json::json!({"key": "value"}));
        
        let fields = doc.get_all_text_fields();
        assert_eq!(fields.len(), 1);
        assert!(fields.contains(&"Test Title".to_string()));
        // Complex types (arrays and objects) should be filtered out
    }
}
