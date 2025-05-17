// src/import_tool.rs - Utility to import existing quantum search index to new database format

use crate::database::{DocumentDatabase, StoredDocument, prime_vector_to_document};
use crate::prime_hilbert::{PrimeVector, BiorthogonalVector};
use std::path::Path;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::collections::HashMap;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json;

/// Import tool for old data formats
pub struct ImportTool {
    db: DocumentDatabase,
    imported_count: usize,
}

impl ImportTool {
    /// Create a new import tool
    pub fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let db = DocumentDatabase::new(db_path)?;
        
        Ok(ImportTool {
            db,
            imported_count: 0,
        })
    }
    
    /// Import from a checkpoint file
    pub fn import_from_checkpoint(&mut self, checkpoint_path: &str) -> Result<usize, Box<dyn std::error::Error>> {
        println!("Importing from checkpoint file: {}", checkpoint_path);
        
        let file = File::open(checkpoint_path)?;
        let reader = BufReader::new(file);
        
        let mut imported = 0;
        let mut line_number = 0;
        
        for line in reader.lines() {
            line_number += 1;
            let line = line?;
            
            // Skip header lines
            if line.starts_with('#') {
                continue;
            }
            
            // Parse line
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 5 {
                println!("Warning: Invalid line format at line {}: {}", line_number, line);
                continue;
            }
            
            // Extract fields
            let url = parts[0].to_string();
            let title = parts[1].to_string();
            let entropy: f64 = parts[2].parse().unwrap_or(0.0);
            let reversibility: f64 = parts[3].parse().unwrap_or(1.0);
            let timestamp: u64 = parts[4].parse().unwrap_or(0);
            
            // Create placeholder document (without actual text content)
            // Real text would need to be fetched or provided in a more complete format
            let placeholder_text = format!("Imported document from {}", url);
            
            // Compress text
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(placeholder_text.as_bytes())?;
            let compressed_text = encoder.finish()?;
            
            // Create basic vector representations
            let tokens = vec![1, 3, 5, 7, 11]; // Placeholder primes
            let vector = self.create_placeholder_vector(&tokens);
            let biorthogonal = self.create_placeholder_biorthogonal();
            
            // Calculate buffering (persistence metric)
            let buffering = 0.5; // Default value
            
            // Create stored document
            let doc = StoredDocument {
                id: None,
                url,
                title,
                text_snippet: placeholder_text.clone(),
                compressed_text,
                vector_data: serde_json::to_string(&vector)?,
                biorthogonal_data: serde_json::to_string(&biorthogonal)?,
                entropy,
                reversibility,
                buffering,
                timestamp,
            };
            
            // Store in database
            self.db.store_document(&doc)?;
            imported += 1;
            
            if imported % 100 == 0 {
                println!("Imported {} documents", imported);
            }
        }
        
        self.imported_count += imported;
        println!("Successfully imported {} documents from checkpoint", imported);
        
        Ok(imported)
    }
    
    /// Import from the original index export CSV
    pub fn import_from_csv(&mut self, csv_path: &str) -> Result<usize, Box<dyn std::error::Error>> {
        println!("Importing from CSV file: {}", csv_path);
        
        let mut reader = csv::Reader::from_path(csv_path)?;
        let mut imported = 0;
        
        // Begin transaction for better performance
        self.db.begin_transaction()?;
        
        for result in reader.records() {
            let record = result?;
            
            if record.len() < 3 {
                continue; // Skip invalid records
            }
            
            // Extract fields (format may vary based on your export format)
            let url = record[0].to_string();
            let title = record[1].to_string();
            let entropy: f64 = record[2].parse().unwrap_or(0.0);
            
            // Use default values for fields not in the CSV
            let reversibility = 1.0;
            let buffering = 0.5;
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
                
            // Create placeholder content
            let placeholder_text = format!("Imported document from {}", url);
            
            // Compress text
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(placeholder_text.as_bytes())?;
            let compressed_text = encoder.finish()?;
            
            // Create basic vector representations
            let tokens = vec![1, 3, 5, 7, 11]; // Placeholder primes
            let vector = self.create_placeholder_vector(&tokens);
            let biorthogonal = self.create_placeholder_biorthogonal();
            
            // Create stored document
            let doc = StoredDocument {
                id: None,
                url,
                title,
                text_snippet: placeholder_text.clone(),
                compressed_text,
                vector_data: serde_json::to_string(&vector)?,
                biorthogonal_data: serde_json::to_string(&biorthogonal)?,
                entropy,
                reversibility,
                buffering,
                timestamp,
            };
            
            // Store in database
            self.db.store_document(&doc)?;
            imported += 1;
            
            if imported % 100 == 0 {
                println!("Imported {} documents", imported);
            }
        }
        
        // Commit transaction
        self.db.commit_transaction()?;
        
        self.imported_count += imported;
        println!("Successfully imported {} documents from CSV", imported);
        
        Ok(imported)
    }
    
    /// Create a custom import from a specific format
    pub fn custom_import<P: AsRef<Path>>(
        &mut self,
        path: P,
        format_type: &str
    ) -> Result<usize, Box<dyn std::error::Error>> {
        match format_type {
            "json" => self.import_from_json(path.as_ref()),
            "xml" => self.import_from_xml(path.as_ref()),
            "custom" => self.import_from_custom(path.as_ref()),
            _ => Err(Box::new(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unsupported format type: {}", format_type)
            ))),
        }
    }
    
    /// Import from JSON format
    fn import_from_json(&mut self, path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
        println!("Importing from JSON file: {}", path.display());
        
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        // Parse JSON
        let data: Vec<serde_json::Value> = serde_json::from_reader(reader)?;
        let mut imported = 0;
        
        // Begin transaction
        self.db.begin_transaction()?;
        
        for item in data {
            // Extract fields from JSON
            let url = item["url"].as_str().unwrap_or("unknown").to_string();
            let title = item["title"].as_str().unwrap_or("Untitled").to_string();
            let text = item["text"].as_str().unwrap_or("").to_string();
            let entropy = item["entropy"].as_f64().unwrap_or(0.0);
            let reversibility = item["reversibility"].as_f64().unwrap_or(1.0);
            let buffering = item["buffering"].as_f64().unwrap_or(0.5);
            let timestamp = item["timestamp"].as_u64().unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            });
            
            // Extract vector data if available
            let vector = if let Some(vec_data) = item["vector"].as_object() {
                let mut vector = HashMap::new();
                for (key, value) in vec_data {
                    if let Ok(prime) = key.parse::<u64>() {
                        if let Some(weight) = value.as_f64() {
                            vector.insert(prime, weight);
                        }
                    }
                }
                vector
            } else {
                // Create placeholder vector
                self.create_placeholder_vector(&[1, 3, 5, 7, 11])
            };
            
            // Create biorthogonal vector
            let biorthogonal = if let Some(bio_data) = item["biorthogonal"].as_object() {
                // Extract real biorthogonal data
                let mut left = HashMap::new();
                let mut right = HashMap::new();
                
                if let Some(left_data) = bio_data.get("left").and_then(|v| v.as_object()) {
                    for (key, value) in left_data {
                        if let Ok(prime) = key.parse::<u64>() {
                            if let Some(weight) = value.as_f64() {
                                left.insert(prime, weight);
                            }
                        }
                    }
                }
                
                if let Some(right_data) = bio_data.get("right").and_then(|v| v.as_object()) {
                    for (key, value) in right_data {
                        if let Ok(prime) = key.parse::<u64>() {
                            if let Some(weight) = value.as_f64() {
                                right.insert(prime, weight);
                            }
                        }
                    }
                }
                
                BiorthogonalVector { left, right }
            } else {
                // Create placeholder biorthogonal
                self.create_placeholder_biorthogonal()
            };
            
            // Compress text
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(text.as_bytes())?;
            let compressed_text = encoder.finish()?;
            
            // Create snippet
            let snippet = if text.len() > 200 {
                text[..200].to_string() + "..."
            } else {
                text.clone()
            };
            
            // Create stored document
            let doc = StoredDocument {
                id: None,
                url,
                title,
                text_snippet: snippet,
                compressed_text,
                vector_data: serde_json::to_string(&vector)?,
                biorthogonal_data: serde_json::to_string(&biorthogonal)?,
                entropy,
                reversibility,
                buffering,
                timestamp,
            };
            
            // Store in database
            self.db.store_document(&doc)?;
            imported += 1;
            
            if imported % 100 == 0 {
                println!("Imported {} documents", imported);
            }
        }
        
        // Commit transaction
        self.db.commit_transaction()?;
        
        self.imported_count += imported;
        println!("Successfully imported {} documents from JSON", imported);
        
        Ok(imported)
    }
    
    /// Import from XML format
    fn import_from_xml(&mut self, path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
        // XML parsing logic would go here
        // For brevity, not implemented in this example
        
        println!("XML import not yet implemented for: {}", path.display());
        Ok(0)
    }
    
    /// Import from custom format
    fn import_from_custom(&mut self, path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
        // Custom format parsing logic would go here
        // For brevity, not implemented in this example
        
        println!("Custom import not yet implemented for: {}", path.display());
        Ok(0)
    }
    
    /// Get total imported count
    pub fn get_imported_count(&self) -> usize {
        self.imported_count
    }
    
    /// Create a placeholder vector for importing
    fn create_placeholder_vector(&self, tokens: &[u64]) -> PrimeVector {
        let mut vector = HashMap::new();
        let n = tokens.len() as f64;
        
        // Create a simple normalized vector
        for (i, &token) in tokens.iter().enumerate() {
            vector.insert(token, 1.0 / (i as f64 + 1.0) / n);
        }
        
        vector
    }
    
    /// Create a placeholder biorthogonal vector
    fn create_placeholder_biorthogonal(&self) -> BiorthogonalVector {
        let base_primes = vec![2, 3, 5, 7, 11, 13];
        let n = base_primes.len() as f64;
        
        let mut left = HashMap::new();
        let mut right = HashMap::new();
        
        // Create simple normalized vectors
        for (i, &prime) in base_primes.iter().enumerate() {
            left.insert(prime, 1.0 / (i as f64 + 1.0) / n);
            right.insert(prime, 0.9 / (i as f64 + 1.0) / n);
        }
        
        BiorthogonalVector { left, right }
    }
}