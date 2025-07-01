// src/main.rs - Clean HDD Search Engine

mod tokenizer;
mod entropy;
mod prime_hilbert;
mod engine;
mod crawler;
mod quantum_types;

use engine::ResonantEngine;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=====================================================");
    println!("🔍 RESONANT HDD SEARCH ENGINE 🔍");
    println!("Find files by content, not just name!");
    println!("=====================================================");

    let mut engine = ResonantEngine::new();

    // Get search path from user
    println!("\nEnter search path (e.g., D:/ or D:/code or C:/Users):");
    print!("> ");
    io::stdout().flush()?;

    let mut search_path = String::new();
    io::stdin().read_line(&mut search_path)?;
    let search_path = search_path.trim();

    if search_path.is_empty() {
        println!("No path entered. Exiting.");
        return Ok(());
    }

    let path = Path::new(search_path);
    if !path.exists() {
        println!("❌ Path '{}' doesn't exist!", search_path);
        return Ok(());
    }

    // Get max depth
    println!("\nMax directory depth to search (default: 10, max: 50):");
    print!("> ");
    io::stdout().flush()?;

    let mut depth_input = String::new();
    io::stdin().read_line(&mut depth_input)?;
    let max_depth: usize = depth_input.trim().parse().unwrap_or(10).min(50);

    // Get max files
    println!("\nMax files to index (default: 5000, recommended max: 50000):");
    print!("> ");
    io::stdout().flush()?;

    let mut files_input = String::new();
    io::stdin().read_line(&mut files_input)?;
    let max_files: usize = files_input.trim().parse().unwrap_or(5000).min(50000);

    // Get number of workers
    println!("\nNumber of worker threads (default: 8, max: 32):");
    print!("> ");
    io::stdout().flush()?;

    let mut workers_input = String::new();
    io::stdin().read_line(&mut workers_input)?;
    let num_workers: usize = workers_input.trim().parse().unwrap_or(8).min(32).max(1);

    println!("\n🚀 Starting HDD scan...");
    println!("📁 Path: {}", search_path);
    println!("📊 Max depth: {}", max_depth);
    println!("📄 Max files: {}", max_files);
    println!("⚡ Workers: {}", num_workers);
    println!();

    let start_time = Instant::now();
    
    // Start the deep directory scan
    match engine.scan_filesystem(path, max_depth, max_files, num_workers) {
        Ok(indexed_count) => {
            let elapsed = start_time.elapsed();
            println!("✅ Indexing complete!");
            println!("📄 Total files indexed: {}", indexed_count);
            println!("⏱️  Time taken: {:.2} seconds", elapsed.as_secs_f64());
            println!("⚡ Average speed: {:.1} files/sec", indexed_count as f64 / elapsed.as_secs_f64());
        }
        Err(e) => {
            eprintln!("❌ Error during indexing: {}", e);
            println!("Continuing with whatever was indexed...");
        }
    }

    if engine.len() == 0 {
        println!("❌ No files were indexed. Check your path and permissions.");
        return Ok(());
    }

    println!("\n🎯 SEARCH MODE ACTIVATED");
    println!("Now you can search by content, concepts, or keywords!");

    // Interactive search loop
    loop {
        println!("\n🔮 Enter your search query (or 'quit' to exit):");
        print!("> ");
        io::stdout().flush()?;

        let mut query = String::new();
        io::stdin().read_line(&mut query)?;
        let query = query.trim();

        if query.eq_ignore_ascii_case("quit") || query.eq_ignore_ascii_case("exit") {
            println!("🌟 Search complete. May the resonance be with you!");
            break;
        }

        if query.is_empty() {
            continue;
        }

        println!("\n🔍 Searching {} indexed files...", engine.len());
        let search_start = Instant::now();
        let results = engine.search(query, 10); // Show top 10 results
        let search_time = search_start.elapsed();

        println!("\n🎯 TOP RESONANT MATCHES (search took {:.3}s):", search_time.as_secs_f64());
        
        if results.is_empty() {
            println!("❌ No resonant patterns found.");
            println!("💡 Try different keywords or concepts.");
        } else {
            for (idx, result) in results.iter().enumerate() {
                println!("\n[{}] 📄 {}", idx + 1, result.title);
                println!("    📍 Path: {}", result.path);
                println!("    🎵 Resonance:      {:.4}", result.resonance);
                println!("    🌀 Δ Entropy:      {:.4}", result.delta_entropy);
                println!("    ⭐ Score:          {:.4}", result.score);
                println!("    🔮 Quantum:        {:.4}", result.quantum_score);
                println!("    🌊 Persistence:    {:.4}", result.persistence_score);
                println!("    👁️  Preview:        {}", result.snippet);
            }
        }

        // Show some stats
        println!("\n📊 Search Stats:");
        println!("    Files searched: {}", engine.len());
        println!("    Results found: {}", results.len());
        println!("    Search time: {:.3}s", search_time.as_secs_f64());
    }

    Ok(())
}