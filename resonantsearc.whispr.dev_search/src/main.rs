v// src/main.rs - Web Search Engine Version

mod tokenizer;
mod entropy;
mod prime_hilbert;
mod engine;
mod crawler;
mod quantum_types;
mod database;
mod search_api;
mod web_server;
mod advanced_crawler;
mod import_tool;

use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use url::Url;
use clap::{App, Arg, SubCommand};
use search_api::{SearchAPI, SearchConfig};
use database::{DocumentDatabase, StoredDocument, prime_vector_to_document};
use crawler::CrawledDocument;
use advanced_crawler::AdvancedCrawler;
use web_server::start_server;
use import_tool::ImportTool;

// Document processor that handles converting crawled documents to database entries
struct DocumentProcessor {
    tokenizer: Arc<Mutex<tokenizer::PrimeTokenizer>>,
    db: Arc<Mutex<DocumentDatabase>>,
    processed_count: Arc<Mutex<usize>>,
}

impl DocumentProcessor {
    fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let db = DocumentDatabase::new(db_path)?;
        
        Ok(DocumentProcessor {
            tokenizer: Arc::new(Mutex::new(tokenizer::PrimeTokenizer::new())),
            db: Arc::new(Mutex::new(db)),
            processed_count: Arc::new(Mutex::new(0)),
        })
    }
    
    async fn process_document(&self, doc: CrawledDocument) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if doc.text.trim().is_empty() {
            println!("Skipping empty document: {}", doc.url);
            return Ok(());
        }
        
        // Tokenize text
        let tokens = {
            let mut tokenizer = self.tokenizer.lock().unwrap();
            tokenizer.tokenize(&doc.text)
        };
        
        if tokens.is_empty() {
            println!("Skipping document with no tokens: {}", doc.url);
            return Ok(());
        }
        
        // Calculate vector representations
        let vector = prime_hilbert::build_vector(&tokens);
        let biorthogonal = prime_hilbert::build_biorthogonal_vector(&tokens);
        let entropy = entropy::shannon_entropy(&tokens);
        
        // Convert to dense vector for historical comparisons
        let dense_vec = prime_hilbert::to_dense_vector(&vector, 1000);
        
        // Calculate persistence metrics
        let reversibility = 1.0; // New document is fully reversible with itself
        let buffering = entropy::buffering_capacity(&dense_vec);
        
        // Compress text
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(doc.text.as_bytes())?;
        let compressed_text = encoder.finish()?;
        
        // Create stored document
        let stored_doc = prime_vector_to_document(
            doc.url,
            doc.title,
            doc.text.clone(),
            compressed_text,
            vector,
            biorthogonal,
            entropy,
            reversibility,
            buffering,
        )?;
        
        // Store in database
        {
            let mut db = self.db.lock().unwrap();
            db.store_document(&stored_doc)?;
            
            // Update count
            let mut count = self.processed_count.lock().unwrap();
            *count += 1;
            
            if *count % 10 == 0 {
                println!("Processed {} documents", *count);
            }
        }
        
        Ok(())
    }
    
    fn get_processed_count(&self) -> usize {
        *self.processed_count.lock().unwrap()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup command line interface
    let matches = App::new("Resonant Search Engine")
        .version("1.0.0")
        .author("RYO Modular")
        .about("Quantum-inspired web search engine")
        .subcommand(
            SubCommand::with_name("crawl")
                .about("Crawl websites and build index")
                .arg(Arg::with_name("urls")
                     .short("u")
                     .long("urls")
                     .value_name("FILE")
                     .help("File containing seed URLs (one per line)")
                     .takes_value(true))
                .arg(Arg::with_name("domain")
                     .short("d")
                     .long("domain")
                     .value_name("DOMAIN")
                     .help("Single domain to crawl")
                     .takes_value(true))
                .arg(Arg::with_name("limit")
                     .short("l")
                     .long("limit")
                     .value_name("NUM")
                     .help("Maximum number of pages to crawl")
                     .default_value("1000")
                     .takes_value(true))
                .arg(Arg::with_name("depth")
                     .short("m")
                     .long("max-depth")
                     .value_name("NUM")
                     .help("Maximum crawl depth")
                     .default_value("3")
                     .takes_value(true))
                .arg(Arg::with_name("workers")
                     .short("w")
                     .long("workers")
                     .value_name("NUM")
                     .help("Number of concurrent crawlers")
                     .default_value("10")
                     .takes_value(true))
                .arg(Arg::with_name("stay-in-domain")
                     .long("stay-in-domain")
                     .help("Stay within the initial domain(s)"))
                .arg(Arg::with_name("db-path")
                     .long("db-path")
                     .value_name("PATH")
                     .help("Path to the database file")
                     .default_value("data/search_db.sqlite")
                     .takes_value(true))
        )
        .subcommand(
            SubCommand::with_name("search")
                .about("Interactive search mode")
                .arg(Arg::with_name("db-path")
                     .long("db-path")
                     .value_name("PATH")
                     .help("Path to the database file")
                     .default_value("data/search_db.sqlite")
                     .takes_value(true))
                .arg(Arg::with_name("disable-quantum")
                     .long("disable-quantum")
                     .help("Disable quantum-inspired scoring"))
                .arg(Arg::with_name("disable-persistence")
                     .long("disable-persistence")
                     .help("Disable persistence-based scoring"))
        )
        .subcommand(
            SubCommand::with_name("serve")
                .about("Start the web server")
                .arg(Arg::with_name("port")
                     .short("p")
                     .long("port")
                     .value_name("PORT")
                     .help("Port to listen on")
                     .default_value("8080")
                     .takes_value(true))
                .arg(Arg::with_name("db-path")
                     .long("db-path")
                     .value_name("PATH")
                     .help("Path to the database file")
                     .default_value("data/search_db.sqlite")
                     .takes_value(true))
                .arg(Arg::with_name("disable-quantum")
                     .long("disable-quantum")
                     .help("Disable quantum-inspired scoring"))
                .arg(Arg::with_name("disable-persistence")
                     .long("disable-persistence")
                     .help("Disable persistence-based scoring"))
        )
        .subcommand(
            SubCommand::with_name("import")
                .about("Import existing index data")
                .arg(Arg::with_name("source")
                     .short("s")
                     .long("source")
                     .value_name("PATH")
                     .help("Path to the source data file")
                     .required(true)
                     .takes_value(true))
                .arg(Arg::with_name("format")
                     .short("f")
                     .long("format")
                     .value_name("FORMAT")
                     .help("Format of the source data (checkpoint, csv, json, xml, custom)")
                     .default_value("checkpoint")
                     .takes_value(true))
                .arg(Arg::with_name("db-path")
                     .long("db-path")
                     .value_name("PATH")
                     .help("Path to the database file")
                     .default_value("data/search_db.sqlite")
                     .takes_value(true))
        )
        .get_matches();

    // Handle subcommands
    match matches.subcommand() {
        ("crawl", Some(crawl_matches)) => {
            run_crawler(crawl_matches).await?;
        },
        ("search", Some(search_matches)) => {
            run_search_mode(search_matches).await?;
        },
        ("serve", Some(serve_matches)) => {
            run_web_server(serve_matches).await?;
        },
        ("import", Some(import_matches)) => {
            run_import(import_matches).await?;
        },
        _ => {
            println!("No subcommand provided. Use --help to see available commands.");
        }
    }

    Ok(())
}

// Run the import tool
async fn run_import(matches: &clap::ArgMatches<'_>) -> Result<(), Box<dyn std::error::Error>> {
    let source_path = matches.value_of("source").unwrap();
    let format = matches.value_of("format").unwrap_or("checkpoint");
    let db_path = matches.value_of("db-path").unwrap_or("data/search_db.sqlite");
    
    println!("Importing from {} (format: {}) to {}", source_path, format, db_path);
    
    // Ensure database directory exists
    if let Some(parent) = Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Create import tool
    let mut import_tool = ImportTool::new(db_path)?;
    
    // Import based on format
    match format {
        "checkpoint" => {
            let count = import_tool.import_from_checkpoint(source_path)?;
            println!("Successfully imported {} documents from checkpoint", count);
        },
        "csv" => {
            let count = import_tool.import_from_csv(source_path)?;
            println!("Successfully imported {} documents from CSV", count);
        },
        "json" => {
            let count = import_tool.custom_import(source_path, "json")?;
            println!("Successfully imported {} documents from JSON", count);
        },
        "xml" => {
            let count = import_tool.custom_import(source_path, "xml")?;
            println!("Successfully imported {} documents from XML", count);
        },
        "custom" => {
            let count = import_tool.custom_import(source_path, "custom")?;
            println!("Successfully imported {} documents from custom format", count);
        },
        _ => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput, 
                format!("Unsupported format: {}", format)
            )));
        }
    }
    
    println!("Import completed. Total imported: {}", import_tool.get_imported_count());
    
    Ok(())
}

// Run the web crawler
async fn run_crawler(matches: &clap::ArgMatches<'_>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting web crawler...");
    
    // Parse crawler settings
    let page_limit = matches.value_of("limit").unwrap_or("1000").parse::<usize>().unwrap_or(1000);
    let max_depth = matches.value_of("depth").unwrap_or("3").parse::<u32>().unwrap_or(3);
    let num_workers = matches.value_of("workers").unwrap_or("10").parse::<usize>().unwrap_or(10);
    let stay_in_domain = matches.is_present("stay-in-domain");
    let db_path = matches.value_of("db-path").unwrap_or("data/search_db.sqlite");
    
    // Ensure database directory exists
    if let Some(parent) = Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Get seed URLs
    let seed_urls = if let Some(url_file) = matches.value_of("urls") {
        // Load URLs from file
        load_urls_from_file(url_file)?
    } else if let Some(domain) = matches.value_of("domain") {
        // Use single domain
        let domain_url = if domain.starts_with("http") {
            domain.to_string()
        } else {
            format!("https://{}", domain)
        };
        
        match Url::parse(&domain_url) {
            Ok(_) => vec![domain_url],
            Err(e) => {
                eprintln!("Invalid URL: {}. Error: {}", domain_url, e);
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid domain URL")));
            }
        }
    } else {
        // Interactive mode
        println!("No seed URLs provided. Please enter seed URLs (one per line, empty line to finish):");
        let mut urls = Vec::new();
        loop {
            print!("> ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            
            let input = input.trim();
            if input.is_empty() {
                break;
            }
            
            // Validate URL
            if let Err(e) = Url::parse(input) {
                println!("Invalid URL: {}. Error: {}", input, e);
                continue;
            }
            
            urls.push(input.to_string());
        }
        
        if urls.is_empty() {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "No seed URLs provided")));
        }
        
        urls
    };
    
    println!("Starting crawl with {} seed URLs", seed_urls.len());
    println!("Max pages: {}, Max depth: {}, Workers: {}", page_limit, max_depth, num_workers);
    if stay_in_domain {
        println!("Staying within initial domain(s)");
    }
    
    // Setup document processor
    let processor = DocumentProcessor::new(db_path)?;
    
    // Setup channels
    let (doc_sender, mut doc_receiver) = mpsc::channel::<CrawledDocument>(100);
    
    let mut crawler = AdvancedCrawler::new(
    doc_sender.clone(),
    num_workers,
    "ResonantSearch/1.0 (+https://whispr.dev/resonant-search/bot.html)",
    );
    
    crawler.configure(page_limit, max_depth, true, true, 1000);
    
    if stay_in_domain {
        // Extract domains from URLs
        let domains: Vec<String> = seed_urls
            .iter()
            .filter_map(|url| {
                Url::parse(url).ok().and_then(|u| u.host_str().map(|h| h.to_string()))
            })
            .collect();
            
        crawler.set_allowed_domains(domains);
    }
    
    // Start crawler in background
    let crawler_handle = tokio::spawn(async move {
        if let Err(e) = crawler.crawl(seed_urls).await {
            eprintln!("Crawler error: {}", e);
        }
        // Close the channel when done
        drop(doc_sender);
    });
    
    // Process documents as they arrive
    while let Some(doc) = doc_receiver.recv().await {
        if let Err(e) = processor.process_document(doc).await {
            eprintln!("Error processing document: {}", e);
        }
    }
    
    // Wait for crawler to finish
    crawler_handle.await?;
    
    let processed_count = processor.get_processed_count();
    println!("Crawling completed. Processed {} documents.", processed_count);
    
    Ok(())
}

// Run interactive search mode
async fn run_search_mode(matches: &clap::ArgMatches<'_>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting interactive search mode...");
    
    let db_path = matches.value_of("db-path").unwrap_or("data/search_db.sqlite");
    let use_quantum = !matches.is_present("disable-quantum");
    let use_persistence = !matches.is_present("disable-persistence");
    
    // Create search API
    let mut search_api = SearchAPI::new(db_path)?;
    
    // Configure search settings
    search_api.configure(
        use_quantum,
        use_persistence,
        0.1, // entropy weight
        0.2, // fragility
        0.05, // trend decay
    );
    
    let doc_count = search_api.count_documents()?;
    println!("Ready. Database contains {} documents.", doc_count);
    
    if use_quantum {
        println!("Quantum-inspired scoring enabled");
    } else {
        println!("Quantum-inspired scoring disabled");
    }
    
    if use_persistence {
        println!("Persistence-based scoring enabled");
    } else {
        println!("Persistence-based scoring disabled");
    }
    
    // Search loop
    loop {
        println!("\nEnter your search query (or 'quit' to exit):");
        print!("> ");
        io::stdout().flush()?;
        
        let mut query = String::new();
        io::stdin().read_line(&mut query)?;
        let query = query.trim();
        
        if query.eq_ignore_ascii_case("quit") {
            println!("Exiting search mode.");
            break;
        }
        
        if query.eq_ignore_ascii_case("optimize") {