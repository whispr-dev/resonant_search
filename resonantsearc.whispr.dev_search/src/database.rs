// src/database.rs

use crate::prime_hilbert::{PrimeVector, BiorthogonalVector};
use std::path::Path;
use std::fs;
use std::io;
use rusqlite::{params, Connection, Result as SqlResult, OptionalExtension};
use serde::{Serialize, Deserialize};
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};

/// Document representation for database storage
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredDocument {
    pub id: Option<i64>,
    pub url: String,
    pub title: String,
    pub text_snippet: String,
    pub compressed_text: Vec<u8>,
    pub vector_data: String,         // Serialized PrimeVector
    pub biorthogonal_data: String,   // Serialized BiorthogonalVector
    pub entropy: f64,
    pub reversibility: f64,
    pub buffering: f64,
    pub timestamp: u64,
}

/// Database wrapper for document storage and retrieval
pub struct DocumentDatabase {
    conn: Connection,
}

impl DocumentDatabase {
    /// Create a new database connection
    pub fn new(db_path: &str) -> SqlResult<Self> {
        // Ensure directory exists
        if let Some(parent) = Path::new(db_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Open connection
        let conn = Connection::open(db_path)?;
        
        // Create tables if they don't exist
        Self::initialize_database(&conn)?;
        
        Ok(DocumentDatabase { conn })
    }
    
    /// Initialize database schema
    fn initialize_database(conn: &Connection) -> SqlResult<()> {
        // Documents table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY,
                url TEXT UNIQUE NOT NULL,
                title TEXT NOT NULL,
                text_snippet TEXT NOT NULL,
                compressed_text BLOB NOT NULL,
                vector_data TEXT NOT NULL,
                biorthogonal_data TEXT NOT NULL,
                entropy REAL NOT NULL,
                reversibility REAL NOT NULL,
                buffering REAL NOT NULL,
                timestamp INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;
        
        // Create full-text search index
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS document_fts USING fts5(
                url, title, text_snippet, 
                content='documents', 
                content_rowid='id',
                tokenize='porter unicode61'
            )",
            [],
        )?;
        
        // Create trigger to update FTS index when documents are inserted
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
                INSERT INTO document_fts(rowid, url, title, text_snippet) 
                VALUES (new.id, new.url, new.title, new.text_snippet);
            END",
            [],
        )?;
        
        // Create trigger for document updates
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS documents_au AFTER UPDATE ON documents BEGIN
                INSERT INTO document_fts(document_fts, rowid, url, title, text_snippet) 
                VALUES('delete', old.id, old.url, old.title, old.text_snippet);
                INSERT INTO document_fts(rowid, url, title, text_snippet) 
                VALUES (new.id, new.url, new.title, new.text_snippet);
            END",
            [],
        )?;
        
        // Create trigger for document deletions
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS documents_ad AFTER DELETE ON documents BEGIN
                INSERT INTO document_fts(document_fts, rowid, url, title, text_snippet) 
                VALUES('delete', old.id, old.url, old.title, old.text_snippet);
            END",
            [],
        )?;
        
        // Create indices for faster queries
        conn.execute("CREATE INDEX IF NOT EXISTS idx_documents_url ON documents(url)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_documents_timestamp ON documents(timestamp)", [])?;
        
        Ok(())
    }
    
    /// Store a document in the database
    pub fn store_document(&self, document: &StoredDocument) -> SqlResult<i64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        let result = self.conn.execute(
            "INSERT OR REPLACE INTO documents (
                url, title, text_snippet, compressed_text, 
                vector_data, biorthogonal_data, entropy,
                reversibility, buffering, timestamp, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                document.url,
                document.title,
                document.text_snippet,
                document.compressed_text,
                document.vector_data,
                document.biorthogonal_data,
                document.entropy,
                document.reversibility,
                document.buffering,
                document.timestamp,
                now
            ],
        )?;
        
        Ok(self.conn.last_insert_rowid())
    }
    
    /// Retrieve a document by URL
    pub fn get_document_by_url(&self, url: &str) -> SqlResult<Option<StoredDocument>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, text_snippet, compressed_text, 
                    vector_data, biorthogonal_data, entropy,
                    reversibility, buffering, timestamp
             FROM documents 
             WHERE url = ?"
        )?;
        
        stmt.query_row(params![url], |row| {
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
    
    /// Text search using the FTS index
    pub fn text_search(&self, query: &str, limit: usize) -> SqlResult<Vec<StoredDocument>> {
        let mut stmt = self.conn.prepare(
            "SELECT d.id, d.url, d.title, d.text_snippet, d.compressed_text, 
                    d.vector_data, d.biorthogonal_data, d.entropy,
                    d.reversibility, d.buffering, d.timestamp,
                    rank
             FROM document_fts
             JOIN documents d ON document_fts.rowid = d.id
             WHERE document_fts MATCH ?
             ORDER BY rank
             LIMIT ?"
        )?;
        
        let rows = stmt.query_map(params![query, limit as i64], |row| {
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
    
    /// Count total documents
    pub fn count_documents(&self) -> SqlResult<i64> {
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM documents")?;
        stmt.query_row([], |row| row.get(0))
    }
    
    /// Get all document vectors for batch operations
    pub fn get_all_document_vectors(&self) -> SqlResult<Vec<(i64, PrimeVector)>> {
        let mut stmt = self.conn.prepare("SELECT id, vector_data FROM documents")?;
        
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let vector_json: String = row.get(1)?;
            let vector: PrimeVector = serde_json::from_str(&vector_json)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
                
            Ok((id, vector))
        })?;
        
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        
        Ok(results)
    }
    
    /// Update document vector data
    pub fn update_document_vector(&self, id: i64, vector: &PrimeVector) -> SqlResult<()> {
        let vector_json = serde_json::to_string(vector)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
            
        self.conn.execute(
            "UPDATE documents SET vector_data = ? WHERE id = ?",
            params![vector_json, id],
        )?;
        
        Ok(())
    }
    
    /// Update document biorthogonal vector data
    pub fn update_document_biorthogonal(&self, id: i64, bio: &BiorthogonalVector) -> SqlResult<()> {
        let bio_json = serde_json::to_string(bio)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
            
        self.conn.execute(
            "UPDATE documents SET biorthogonal_data = ? WHERE id = ?",
            params![bio_json, id],
        )?;
        
        Ok(())
    }
    
    /// Update document persistence metrics
    pub fn update_document_persistence(
        &self, 
        id: i64, 
        reversibility: f64, 
        buffering: f64
    ) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE documents SET reversibility = ?, buffering = ? WHERE id = ?",
            params![reversibility, buffering, id],
        )?;
        
        Ok(())
    }
    
    /// Begin a database transaction
    pub fn begin_transaction(&self) -> SqlResult<()> {
        self.conn.execute("BEGIN TRANSACTION", [])?;
        Ok(())
    }
    
    /// Commit a database transaction
    pub fn commit_transaction(&self) -> SqlResult<()> {
        self.conn.execute("COMMIT", [])?;
        Ok(())
    }
    
    /// Rollback a database transaction
    pub fn rollback_transaction(&self) -> SqlResult<()> {
        self.conn.execute("ROLLBACK", [])?;
        Ok(())
    }
    
    /// Optimize the database
    pub fn optimize(&self) -> SqlResult<()> {
        // Optimize FTS index
        self.conn.execute("INSERT INTO document_fts(document_fts) VALUES('optimize')", [])?;
        
        // Run VACUUM to reclaim space
        self.conn.execute("VACUUM", [])?;
        
        // Update statistics
        self.conn.execute("ANALYZE", [])?;
        
        Ok(())
    }
}

// Helper functions for document conversion

/// Convert a PrimeVector to a StoredDocument
pub fn prime_vector_to_document(
    url: String,
    title: String,
    text: String,
    compressed_text: Vec<u8>,
    vector: PrimeVector,
    biorthogonal: BiorthogonalVector,
    entropy: f64,
    reversibility: f64,
    buffering: f64,
) -> io::Result<StoredDocument> {
    // Create a snippet from the text
    let snippet = create_snippet(&text, 200);
    
    // Serialize vectors to JSON
    let vector_data = serde_json::to_string(&vector)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        
    let biorthogonal_data = serde_json::to_string(&biorthogonal)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        
    // Get current timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
        
    Ok(StoredDocument {
        id: None,
        url,
        title,
        text_snippet: snippet,
        compressed_text,
        vector_data,
        biorthogonal_data,
        entropy,
        reversibility,
        buffering,
        timestamp,
    })
}

/// Parse a stored document back to usable types
pub fn parse_stored_document(doc: &StoredDocument) -> io::Result<(PrimeVector, BiorthogonalVector)> {
    // Parse vector data
    let vector: PrimeVector = serde_json::from_str(&doc.vector_data)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        
    // Parse biorthogonal data
    let biorthogonal: BiorthogonalVector = serde_json::from_str(&doc.biorthogonal_data)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        
    Ok((vector, biorthogonal))
}

/// Create a snippet from text
fn create_snippet(text: &str, max_length: usize) -> String {
    // Remove extra whitespace
    let cleaned_text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    
    if cleaned_text.len() <= max_length {
        cleaned_text
    } else {
        // Find a reasonable breaking point
        let mut end = max_length;
        while end > 0 && !cleaned_text.is_char_boundary(end) {
            end -= 1;
        }
        
        // Find the last space before the limit
        let mut last_space = end;
        while last_space > 0 && !cleaned_text[..last_space].ends_with(char::is_whitespace) {
            last_space -= 1;
        }
        
        if last_space > max_length / 2 {
            format!("{}...", &cleaned_text[..last_space])
        } else {
            format!("{}...", &cleaned_text[..end])
        }
    }
}