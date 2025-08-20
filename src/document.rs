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

