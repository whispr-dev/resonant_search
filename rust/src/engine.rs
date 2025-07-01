// src/engine.rs - Enhanced with deep filesystem scanning

use crate::tokenizer::PrimeTokenizer;
use crate::prime_hilbert::{build_vector, dot_product, PrimeVector, build_biorthogonal_vector, BiorthogonalVector, resonance_complex};
use crate::entropy::{shannon_entropy, entropy_pressure, persistence_score};
use crate::crawler::CrawledDocument;

use std::fs;
use std::path::{Path, PathBuf};
use std::io::{self, Write, Read};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;
use serde::{Serialize, Deserialize};

/// Represents a processed document in the engine's index.
#[derive(Serialize, Deserialize)]
struct IndexedDocument {
    title: String,
    text: String,
    compressed_text: Option<Vec<u8>>,
    vector: PrimeVector,
    biorthogonal: BiorthogonalVector,
    entropy: f64,
    path: PathBuf,
    timestamp: u64,
    // Persistence theory metrics
    reversibility: f64,
    buffering: f64,
    historical_vectors: Vec<Vec<f64>>,
}

impl IndexedDocument {
    /// Compress the document text to save memory
    fn compress_text(&mut self) {
        if !self.text.is_empty() && self.compressed_text.is_none() {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            if encoder.write_all(self.text.as_bytes()).is_ok() {
                if let Ok(compressed) = encoder.finish() {
                    self.compressed_text = Some(compressed);
                    self.text.clear(); // Clear original text to save memory
                }
            }
        }
    }

    /// Decompress the document text for display or further processing
    fn decompress_text(&self) -> String {
        if let Some(compressed_bytes) = &self.compressed_text {
            let mut decoder = GzDecoder::new(&compressed_bytes[..]);
            let mut decompressed_text = String::new();
            if decoder.read_to_string(&mut decompressed_text).is_ok() {
                decompressed_text
            } else {
                self.text.clone()
            }
        } else {
            self.text.clone()
        }
    }
}

/// Represents a search result with scoring details and a snippet.
#[derive(Debug)]
pub struct SearchResult {
    pub title: String,
    pub snippet: String,
    pub resonance: f64,
    pub delta_entropy: f64,
    pub score: f64,
    pub quantum_score: f64,
    pub persistence_score: f64,
    pub path: String,
}

pub struct ResonantEngine {
    tokenizer: PrimeTokenizer,
    documents: Vec<IndexedDocument>,
    use_quantum_score: bool,
    use_persistence_score: bool,
    // Persistence theory parameters
    fragility: f64,
    entropy_weight: f64,
}

impl ResonantEngine {
    pub fn new() -> Self {
        ResonantEngine {
            tokenizer: PrimeTokenizer::new(),
            documents: Vec::new(),
            use_quantum_score: true,    // Enable by default
            use_persistence_score: true, // Enable by default
            fragility: 0.2,
            entropy_weight: 0.1,
        }
    }

    pub fn len(&self) -> usize {
        self.documents.len()
    }

    pub fn add_document(&mut self, title: String, text: String, path: PathBuf) {
        if text.trim().is_empty() {
            return; // Skip empty documents
        }
        
        let tokens = self.tokenizer.tokenize(&text);
        let vector = build_vector(&tokens);
        let biorthogonal = build_biorthogonal_vector(&tokens);
        let entropy = shannon_entropy(&tokens);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut doc = IndexedDocument {
            title,
            text,
            compressed_text: None,
            vector,
            biorthogonal,
            entropy,
            path,
            timestamp: now,
            reversibility: 0.5,
            buffering: 0.0,
            historical_vectors: Vec::new(),
        };
        
        doc.compress_text(); // Compress immediately to save memory
        self.documents.push(doc);
    }

    /// Deep filesystem scanning with configurable depth and file limits
    pub fn scan_filesystem<P: AsRef<Path>>(
        &mut self, 
        root_path: P, 
        max_depth: usize, 
        max_files: usize,
        num_workers: usize
    ) -> io::Result<usize> {
        let root = root_path.as_ref();
        println!("🔍 Starting deep scan of: {}", root.display());

        // Supported file extensions
        let supported_extensions = [
            "txt", "md", "rst", "log", "conf", "cfg", "ini", "json", "xml", "csv",
            "html", "htm", "js", "css", "py", "rs", "c", "cpp", "h", "hpp",
            "java", "go", "php", "rb", "sh", "bat", "sql", "yaml", "yml",
            "toml", "dockerfile", "makefile", "readme", "license", "gitignore"
        ];

        // Collect all files first
        let file_paths = Arc::new(Mutex::new(Vec::new()));
        let file_count = Arc::new(Mutex::new(0));
        
        self.collect_files_recursive(
            root, 
            0, 
            max_depth, 
            max_files, 
            &supported_extensions,
            file_paths.clone(),
            file_count.clone()
        )?;

        let paths = file_paths.lock().unwrap().clone();
        let total_files = paths.len();
        
        if total_files == 0 {
            println!("❌ No supported files found in the specified path.");
            return Ok(0);
        }

        println!("📄 Found {} files to index", total_files);

        // Process files with multiple workers
        let (sender, receiver) = mpsc::channel();
        let paths = Arc::new(paths);
        let file_index = Arc::new(Mutex::new(0));

        // Spawn worker threads
        let mut handles = Vec::new();
        for worker_id in 0..num_workers {
            let sender = sender.clone();
            let paths = paths.clone();
            let file_index = file_index.clone();
            let total_files = total_files;

            let handle = thread::spawn(move || {
                loop {
                    // Get next file to process
                    let current_index = {
                        let mut index = file_index.lock().unwrap();
                        if *index >= total_files {
                            break; // No more files
                        }
                        let idx = *index;
                        *index += 1;
                        idx
                    };

                    let file_path = &paths[current_index];
                    
                    // Progress update
                    if current_index % 100 == 0 {
                        println!("Worker {}: Processing file {}/{}", worker_id, current_index + 1, total_files);
                    }

                    // Process the file
                    match Self::process_file(file_path) {
                        Ok(Some((title, content))) => {
                            if let Err(_) = sender.send((title, content, file_path.clone())) {
                                break; // Receiver hung up
                            }
                        }
                        Ok(None) => {
                            // File was empty or couldn't be processed, continue
                        }
                        Err(e) => {
                            eprintln!("Error processing {}: {}", file_path.display(), e);
                        }
                    }
                }
            });
            handles.push(handle);
        }

        // Drop the original sender so receiver knows when all workers are done
        drop(sender);

        // Receive processed documents
        let mut indexed_count = 0;
        while let Ok((title, content, path)) = receiver.recv() {
            self.add_document(title, content, path);
            indexed_count += 1;
            
            if indexed_count % 500 == 0 {
                println!("📚 Indexed {} documents...", indexed_count);
            }
        }

        // Wait for all workers to finish
        for handle in handles {
            handle.join().unwrap();
        }

        println!("✅ Filesystem scan complete!");
        Ok(indexed_count)
    }

    fn collect_files_recursive<P: AsRef<Path>>(
        &self,
        dir: P,
        current_depth: usize,
        max_depth: usize,
        max_files: usize,
        supported_extensions: &[&str],
        file_paths: Arc<Mutex<Vec<PathBuf>>>,
        file_count: Arc<Mutex<usize>>
    ) -> io::Result<()> {
        if current_depth > max_depth {
            return Ok(());
        }

        let entries = fs::read_dir(dir)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Check if we've hit the file limit
            {
                let count = file_count.lock().unwrap();
                if *count >= max_files {
                    return Ok(());
                }
            }

            if path.is_file() {
                // Check if it's a supported file type
                if let Some(extension) = path.extension() {
                    if let Some(ext_str) = extension.to_str() {
                        if supported_extensions.contains(&ext_str.to_lowercase().as_str()) {
                            file_paths.lock().unwrap().push(path);
                            *file_count.lock().unwrap() += 1;
                        }
                    }
                } else {
                    // Files without extensions (like README, Makefile, etc.)
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let name_lower = name.to_lowercase();
                        if name_lower.contains("readme") || 
                           name_lower.contains("license") || 
                           name_lower.contains("makefile") ||
                           name_lower.contains("dockerfile") {
                            file_paths.lock().unwrap().push(path);
                            *file_count.lock().unwrap() += 1;
                        }
                    }
                }
            } else if path.is_dir() {
                // Skip system/hidden directories
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if dir_name.starts_with('.') || 
                       dir_name == "System Volume Information" ||
                       dir_name == "$RECYCLE.BIN" ||
                       dir_name == "node_modules" ||
                       dir_name == "target" ||
                       dir_name == ".git" {
                        continue;
                    }
                }
                
                // Recursively process subdirectory
                if let Err(e) = self.collect_files_recursive(
                    &path, 
                    current_depth + 1, 
                    max_depth, 
                    max_files,
                    supported_extensions,
                    file_paths.clone(),
                    file_count.clone()
                ) {
                    eprintln!("Warning: Could not access directory {}: {}", path.display(), e);
                }
            }
        }

        Ok(())
    }

    fn process_file(path: &Path) -> io::Result<Option<(String, String)>> {
        // Get file title from filename
        let title = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Try to read the file
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => {
                // If UTF-8 reading fails, try reading as bytes and converting
                let bytes = fs::read(path)?;
                String::from_utf8_lossy(&bytes).to_string()
            }
        };

        // Skip if content is too small or too large
        if content.len() < 10 || content.len() > 1_000_000 {
            return Ok(None);
        }

        // Basic content filtering
        let trimmed_content = content.trim();
        if trimmed_content.is_empty() {
            return Ok(None);
        }

        Ok(Some((title, trimmed_content.to_string())))
    }

    pub fn search(&mut self, query: &str, top_n: usize) -> Vec<SearchResult> {
        if self.documents.is_empty() {
            return Vec::new();
        }

        let query_tokens = self.tokenizer.tokenize(query);
        let query_vec = build_vector(&query_tokens);
        let query_entropy = shannon_entropy(&query_tokens);

        let mut results: Vec<SearchResult> = Vec::new();

        for doc in &mut self.documents {
            // Calculate standard resonance and delta entropy
            let resonance = dot_product(&query_vec, &doc.vector);
            let delta_entropy = (query_entropy - doc.entropy).abs();

            // Calculate standard relevance score
            let mut score = resonance - delta_entropy * self.entropy_weight;

            // Calculate quantum score if enabled
            let quantum_score = if self.use_quantum_score {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let doc_age = ((now - doc.timestamp) as f64) / (24.0 * 3600.0);
                let decay_factor = 0.01 * doc_age.min(100.0);
                
                let complex_res = resonance_complex(&query_vec, &doc.vector, decay_factor);
                complex_res.re * 0.6 + complex_res.im.abs() * 0.4
            } else {
                0.0
            };

            // Calculate persistence score if enabled
            let persistence_score = if self.use_persistence_score {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let doc_age = ((now - doc.timestamp) as f64) / (24.0 * 3600.0);
                let update_frequency = 0.1;
                
                let persistence = persistence_score(
                    doc.reversibility,
                    entropy_pressure(doc_age, update_frequency, 0.05),
                    doc.buffering,
                    self.fragility
                );
                
                let entropy_factor = (-delta_entropy * self.entropy_weight).exp();
                persistence * entropy_factor
            } else {
                0.0
            };

            // Apply scoring weights
            if self.use_quantum_score {
                score += quantum_score * 0.3;
            }
            if self.use_persistence_score {
                score += persistence_score * 0.2;
            }

            // Create snippet from decompressed text (Unicode-safe)
            let full_text = doc.decompress_text();
            let snippet = if full_text.chars().count() > 200 {
                let truncated: String = full_text.chars().take(200).collect();
                format!("{}...", truncated)
            } else {
                full_text
            };

            results.push(SearchResult {
                title: doc.title.clone(),
                snippet,
                resonance,
                delta_entropy,
                score,
                quantum_score,
                persistence_score,
                path: doc.path.to_string_lossy().into_owned(),
            });
        }

        // Sort results by combined score (descending)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Return top N results
        results.into_iter().take(top_n).collect()
    }

    // Add the missing methods that were in your original code
    pub fn add_crawled_document(&mut self, doc: CrawledDocument) {
        let path = PathBuf::from(&doc.url);
        self.add_document(doc.title, doc.text, path);
    }

    pub fn set_use_quantum_score(&mut self, enabled: bool) {
        self.use_quantum_score = enabled;
    }

    pub fn set_use_persistence_score(&mut self, enabled: bool) {
        self.use_persistence_score = enabled;
    }

    pub fn set_fragility(&mut self, fragility: f64) {
        self.fragility = fragility;
    }

    pub fn set_entropy_weight(&mut self, weight: f64) {
        self.entropy_weight = weight;
    }
}