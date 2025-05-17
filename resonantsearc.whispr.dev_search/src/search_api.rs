// src/search_api.rs

use crate::engine::{ResonantEngine, SearchResult};
use crate::database::{DocumentDatabase, StoredDocument, parse_stored_document};
use crate::prime_hilbert::{dot_product, resonance_complex, biorthogonal_score};
use crate::entropy::{persistence_score, entropy_pressure, buffering_capacity};
use crate::tokenizer::PrimeTokenizer;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use flate2::read::GzDecoder;
use std::io::Read;

/// Search API that combines traditional FTS (Full-Text Search) with quantum resonance
pub struct SearchAPI {
    db: DocumentDatabase,
    tokenizer: Arc<Mutex<PrimeTokenizer>>,
    use_quantum: bool,
    use_persistence: bool,
    entropy_weight: f64,
    fragility: f64,
    trend_decay: f64,
}

/// Configuration for search operations
pub struct SearchConfig {
    pub limit: usize,
    pub use_quantum: bool,
    pub use_persistence: bool,
    pub hybrid_search: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            limit: 10,
            use_quantum: true,
            use_persistence: true,
            hybrid_search: true,
        }
    }
}

impl SearchAPI {
    /// Create a new SearchAPI instance
    pub fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let db = DocumentDatabase::new(db_path)?;
        let tokenizer = Arc::new(Mutex::new(PrimeTokenizer::new()));
        
        Ok(SearchAPI {
            db,
            tokenizer,
            use_quantum: true,
            use_persistence: true,
            entropy_weight: 0.1,
            fragility: 0.2,
            trend_decay: 0.05,
        })
    }
    
    /// Configure search settings
    pub fn configure(
        &mut self,
        use_quantum: bool,
        use_persistence: bool,
        entropy_weight: f64,
        fragility: f64,
        trend_decay: f64,
    ) -> &mut Self {
        self.use_quantum = use_quantum;
        self.use_persistence = use_persistence;
        self.entropy_weight = entropy_weight;
        self.fragility = fragility;
        self.trend_decay = trend_decay;
        self
    }
    
    /// Search using both text search and quantum resonance
    pub fn search(&self, query: &str, config: SearchConfig) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        // Track search performance
        let start_time = std::time::Instant::now();
        
        // Step 1: Tokenize the query
        let query_tokens = {
            let mut tokenizer = self.tokenizer.lock().unwrap();
            tokenizer.tokenize(query)
        };
        
        if query_tokens.is_empty() {
            return Ok(Vec::new());
        }
        
        // Build query vector from tokens
        let query_vec = crate::prime_hilbert::build_vector(&query_tokens);
        let query_entropy = crate::entropy::shannon_entropy(&query_tokens);
        
        // Step 2: Get initial candidates using text search
        let mut candidates = if config.hybrid_search {
            // Use FTS to get initial candidates
            match self.db.text_search(query, config.limit * 3) {
                Ok(docs) => docs,
                Err(e) => {
                    eprintln!("Text search error: {}", e);
                    Vec::new()
                }
            }
        } else {
            // Get all documents from DB (limited to a reasonable number)
            match self.db.get_all_documents(config.limit * 5) {
                Ok(docs) => docs,
                Err(e) => {
                    eprintln!("Failed to get documents: {}", e);
                    Vec::new()
                }
            }
        };
        
        // Step 3: Score candidates using resonance
        let mut results = Vec::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        for doc in candidates {
            // Parse vector data
            let (vector, biorthogonal) = match parse_stored_document(&doc) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Error parsing document {}: {}", doc.url, e);
                    continue;
                }
            };
            
            // Compute basic resonance score
            let resonance = dot_product(&query_vec, &vector);
            let delta_entropy = (doc.entropy - query_entropy).abs();
            let standard_score = resonance - delta_entropy * self.entropy_weight;
            
            // Compute quantum score if enabled
            let quantum_score = if config.use_quantum && self.use_quantum {
                let doc_age = ((now - doc.timestamp) as f64) / (24.0 * 3600.0); // Age in days
                let decay_factor = 0.01 * doc_age.min(100.0); // Cap at 100 days
                
                let complex_res = resonance_complex(&query_vec, &vector, decay_factor);
                
                // For biorthogonal scoring
                let query_bio = crate::prime_hilbert::build_biorthogonal_vector(&query_tokens);
                let bio_score = biorthogonal_score(&query_bio, &biorthogonal);
                
                // Combine scores
                complex_res.re * 0.6 + complex_res.im.abs() * 0.2 + bio_score * 0.2
            } else {
                0.0
            };
            
            // Compute persistence score if enabled
            let persistence_score_val = if config.use_persistence && self.use_persistence {
                let doc_age = ((now - doc.timestamp) as f64) / (24.0 * 3600.0); // Age in days
                let update_frequency = 0.1; // Lower means less frequent updates
                
                let p_score = persistence_score(
                    doc.reversibility,
                    entropy_pressure(doc_age, update_frequency, self.trend_decay),
                    doc.buffering,
                    self.fragility
                );
                
                let entropy_delta = (doc.entropy - query_entropy).abs();
                let entropy_factor = (-entropy_delta * self.entropy_weight).exp();
                
                p_score * entropy_factor
            } else {
                0.0
            };
            
            // Decompress snippet text if needed
            let snippet = if !doc.text_snippet.is_empty() {
                doc.text_snippet.clone()
            } else if !doc.compressed_text.is_empty() {
                let mut decoder = GzDecoder::new(&doc.compressed_text[..]);
                let mut text = String::new();
                if decoder.read_to_string(&mut text).is_ok() {
                    // Create a snippet
                    let words: Vec<&str> = text.split_whitespace().collect();
                    if words.len() > 30 {
                        words[..30].join(" ") + "..."
                    } else {
                        words.join(" ")
                    }
                } else {
                    "[Content could not be decompressed]".to_string()
                }
            } else {
                "[No content available]".to_string()
            };
            
            results.push(SearchResult {
                title: doc.title,
                resonance,
                delta_entropy,
                score: standard_score,
                quantum_score: quantum_score,
                persistence_score: persistence_score_val,
                snippet,
                path: doc.url,
            });
        }
        
        // Step 4: Sort by combined score
        results.sort_by(|a, b| {
            let a_combined = if config.use_quantum && config.use_persistence {
                a.score * 0.5 + a.quantum_score * 0.25 + a.persistence_score * 0.25
            } else if config.use_quantum {
                a.score * 0.7 + a.quantum_score * 0.3
            } else if config.use_persistence {
                a.score * 0.7 + a.persistence_score * 0.3
            } else {
                a.score
            };
            
            let b_combined = if config.use_quantum && config.use_persistence {
                b.score * 0.5 + b.quantum_score * 0.25 + b.persistence_score * 0.25
            } else if config.use_quantum {
                b.score * 0.7 + b.quantum_score * 0.3
            } else if config.use_persistence {
                b.score * 0.7 + b.persistence_score * 0.3
            } else {
                b.score
            };
            
            b_combined.partial_cmp(&a_combined).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Limit to requested number
        let limited_results = results.into_iter().take(config.limit).collect();
        
        // Log search time
        let elapsed = start_time.elapsed();
        println!("Search '{}' completed in {:?}", query, elapsed);
        
        Ok(limited_results)
    }
    
    /// Apply quantum jump to update document relevance
    pub fn apply_quantum_jump(&self, query: &str, importance: f64) -> Result<(), Box<dyn std::error::Error>> {
        // Tokenize query
        let query_tokens = {
            let mut tokenizer = self.tokenizer.lock().unwrap();
            tokenizer.tokenize(query)
        };
        
        if query_tokens.is_empty() {
            return Ok(());
        }
        
        // Build query vector
        let query_vec = crate::prime_hilbert::build_vector(&query_tokens);
        
        // Get all document vectors
        let docs = match self.db.get_all_document_vectors() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to get document vectors: {}", e);
                return Err(Box::new(e));
            }
        };
        
        // Begin transaction
        self.db.begin_transaction()?;
        
        let mut updated_count = 0;
        
        for (id, vector) in docs {
            // Calculate resonance
            let resonance = dot_product(&query_vec, &vector);
            
            // If the document resonates with the query, boost its relevance
            if resonance > 0.1 {
                // Get document
                if let Ok(Some(mut doc)) = self.db.get_document_by_id(id) {
                    // Update reversibility based on match strength
                    let new_reversibility = doc.reversibility * 0.9 + 0.1 * (resonance * importance);
                    
                    // Update timestamp to mark it as "fresher"
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                        
                    // Only update if significantly newer (> 1 day)
                    if now - doc.timestamp > 24 * 3600 {
                        doc.timestamp = now - ((now - doc.timestamp) / 2); // Make it "halfway" newer
                        
                        // Update document
                        if let Err(e) = self.db.update_document_persistence(id, new_reversibility, doc.buffering) {
                            eprintln!("Failed to update document {}: {}", id, e);
                            continue;
                        }
                        
                        if let Err(e) = self.db.update_document_timestamp(id, doc.timestamp) {
                            eprintln!("Failed to update timestamp for document {}: {}", id, e);
                            continue;
                        }
                        
                        updated_count += 1;
                    }
                }
            }
        }
        
        // Commit transaction
        self.db.commit_transaction()?;
        
        println!("Quantum jump applied to {} documents", updated_count);
        
        Ok(())
    }
    
    /// Get document count
    pub fn count_documents(&self) -> Result<i64, Box<dyn std::error::Error>> {
        Ok(self.db.count_documents()?)
    }
    
    /// Optimize database
    pub fn optimize(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.db.optimize()?;
        Ok(())
    }
}

// Additional methods for DocumentDatabase (add to database.rs)
impl DocumentDatabase {
    /// Get a document by ID
    pub fn get_document_by_id(&self, id: i64) -> rusqlite::Result<Option<StoredDocument>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, text_snippet, compressed_text, 
                    vector_data, biorthogonal_data, entropy,
                    reversibility, buffering, timestamp
             FROM documents 
             WHERE id = ?"
        )?;
        
        stmt.query_row(rusqlite::params![id], |row| {
            Ok(StoredDocument {
                id: Some(row.get(0)?),
                url: row.get(1)?,
                title: row.get(2)?,
                text_snippet: row.get(3)?,
                compressed_text: row.get(4)?,
                vector_data: row.get(5)?,
                biorthogonal_data: row.get(6)?,
                entropy: row.get(7)?,
                reversibility: row.get(8)?,
                buffering: row.get(9)?,
                timestamp: row.get(10)?,
            })
        }).optional()
    }
    
    /// Update document timestamp
    pub fn update_document_timestamp(&self, id: i64, timestamp: u64) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE documents SET timestamp = ? WHERE id = ?",
            rusqlite::params![timestamp, id],
        )?;
        
        Ok(())
    }
    
    /// Get all documents (limited)
    pub fn get_all_documents(&self, limit: usize) -> rusqlite::Result<Vec<StoredDocument>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, text_snippet, compressed_text, 
                    vector_data, biorthogonal_data, entropy,
                    reversibility, buffering, timestamp
             FROM documents
             ORDER BY timestamp DESC
             LIMIT ?"
        )?;
        
        let rows = stmt.query_map(rusqlite::params![limit as i64], |row| {
            Ok(StoredDocument {
                id: Some(row.get(0)?),
                url: row.get(1)?,
                title: row.get(2)?,
                text_snippet: row.get(3)?,
                compressed_text: row.get(4)?,
                vector_data: row.get(5)?,
                biorthogonal_data: row.get(6)?,
                entropy: row.get(7)?,
                reversibility: row.get(8)?,
                buffering: row.get(9)?,
                timestamp: row.get(10)?,
            })
        })?;
        
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        
        Ok(results)
    }
}