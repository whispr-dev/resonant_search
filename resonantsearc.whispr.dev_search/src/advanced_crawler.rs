// src/advanced_crawler.rs

use crate::crawler::CrawledDocument;
use reqwest::{Client, Url, header};
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use tokio::sync::{mpsc, Semaphore};
use std::error::Error;
use std::fmt;
use futures::stream::{self, StreamExt};
use rand::Rng;
use robots_txt::{RobotFileParser, ParsedRobots};

/// A more advanced crawler implementation for web-scale operation
pub struct AdvancedCrawler {
    client: Client,
    doc_sender: mpsc::Sender<CrawledDocument>,
    visited_urls: Arc<Mutex<HashSet<String>>>,
    robots_cache: Arc<Mutex<HashMap<String, (ParsedRobots, u64)>>>, // Cache of robots.txt with timestamp
    concurrency_limiter: Arc<Semaphore>,
    max_pages: usize,
    max_depth: u32,
    allowed_domains: Option<HashSet<String>>,
    domain_rate_limiters: Arc<Mutex<HashMap<String, tokio::sync::Mutex<()>>>>,
    user_agent: String,
    respect_noindex: bool,
    respect_nofollow: bool,
    crawl_delay: u64, // Base delay between requests in milliseconds
}

impl AdvancedCrawler {
    /// Creates a new AdvancedCrawler
    pub fn new(
        doc_sender: mpsc::Sender<CrawledDocument>,
        max_concurrent_requests: usize,
        user_agent: &str,
    ) -> Self {
        // Build HTTP client with appropriate settings for web crawler
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(user_agent)
            .cookie_store(true)
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .pool_max_idle_per_host(2)
            .build()
            .unwrap_or_else(|_| Client::new());

        AdvancedCrawler {
            client,
            doc_sender,
            visited_urls: Arc::new(Mutex::new(HashSet::new())),
            robots_cache: Arc::new(Mutex::new(HashMap::new())),
            concurrency_limiter: Arc::new(Semaphore::new(max_concurrent_requests)),
            max_pages: 10000,
            max_depth: 3,
            allowed_domains: None,
            domain_rate_limiters: Arc::new(Mutex::new(HashMap::new())),
            user_agent: user_agent.to_string(),
            respect_noindex: true,
            respect_nofollow: true,
            crawl_delay: 500, // Default 500ms delay
        }
    }
    
    /// Configure crawler settings
    pub fn configure(
        &mut self,
        max_pages: usize,
        max_depth: u32,
        respect_noindex: bool,
        respect_nofollow: bool,
        crawl_delay: u64,
    ) -> &mut Self {
        self.max_pages = max_pages;
        self.max_depth = max_depth;
        self.respect_noindex = respect_noindex;
        self.respect_nofollow = respect_nofollow;
        self.crawl_delay = crawl_delay;
        self
    }

    /// Set allowed domains for crawling
    pub fn set_allowed_domains(&mut self, domains: Vec<String>) -> &mut Self {
        self.allowed_domains = Some(domains.into_iter().collect());
        self
    }
    
    /// Check if a URL is allowed according to robots.txt
    async fn check_robots(&self, url: &Url) -> bool {
        if let Some(host) = url.host_str() {
            let robots_url = match url.scheme() {
                "https" => format!("https://{}:443/robots.txt", host),
                _ => format!("http://{}:80/robots.txt", host),
            };
            
            // Get or fetch robots.txt
            let robots = {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                    
                let mut cache = self.robots_cache.lock().unwrap();
                
                // Check if cached version exists and is still valid (24 hour TTL)
                if let Some((robots, timestamp)) = cache.get(host) {
                    if now - timestamp < 24 * 3600 {
                        robots.clone()
                    } else {
                        // Expired, fetch new robots.txt
                        drop(cache); // Release lock before async operation
                        match Self::fetch_robots_txt(&self.client, &robots_url).await {
                            Ok(new_robots) => {
                                let mut cache = self.robots_cache.lock().unwrap();
                                cache.insert(host.to_string(), (new_robots.clone(), now));
                                new_robots
                            }
                            Err(_) => {
                                // Error fetching, use empty robots.txt
                                let empty_robots = RobotFileParser::new(&robots_url).parse(&[]).clone();
                                let mut cache = self.robots_cache.lock().unwrap();
                                cache.insert(host.to_string(), (empty_robots.clone(), now));
                                empty_robots
                            }
                        }
                    }
                } else {
                    // Not in cache, fetch new robots.txt
                    drop(cache); // Release lock before async operation
                    match Self::fetch_robots_txt(&self.client, &robots_url).await {
                        Ok(new_robots) => {
                            let mut cache = self.robots_cache.lock().unwrap();
                            cache.insert(host.to_string(), (new_robots.clone(), now));
                            new_robots
                        }
                        Err(_) => {
                            // Error fetching, use empty robots.txt
                            let empty_robots = RobotFileParser::new(&robots_url).parse(&[]).clone();
                            let mut cache = self.robots_cache.lock().unwrap();
                            cache.insert(host.to_string(), (empty_robots.clone(), now));
                            empty_robots
                        }
                    }
                }
            };
            
            // Check if our user agent is allowed to access the URL
            !robots.disallow_all && robots.can_fetch(&self.user_agent, url.path())
        } else {
            // If URL doesn't have a host, we can't check robots.txt
            true
        }
    }
    
    /// Fetch robots.txt file from a URL
    async fn fetch_robots_txt(client: &Client, robots_url: &str) -> Result<ParsedRobots, Box<dyn Error + Send + Sync>> {
        let response = client.get(robots_url).send().await?;
        
        if response.status().is_success() {
            let content = response.text().await?;
            let parser = RobotFileParser::new(robots_url);
            Ok(parser.parse(&content.lines().collect::<Vec<_>>()).clone())
        } else {
            // If robots.txt doesn't exist or can't be accessed, assume everything is allowed
            let parser = RobotFileParser::new(robots_url);
            Ok(parser.parse(&[]).clone())
        }
    }
    
    /// Respect per-domain rate limits
    async fn respect_domain_rate_limit(&self, domain: &str) {
        // Get or create a mutex for this domain
        let rate_limiter = {
            let mut limiters = self.domain_rate_limiters.lock().unwrap();
            if !limiters.contains_key(domain) {
                limiters.insert(domain.to_string(), tokio::sync::Mutex::new(()));
            }
            limiters.get(domain).unwrap().clone()
        };
        
        // Acquire the mutex, which enforces sequential access per domain
        let _guard = rate_limiter.lock().await;
        
        // Add jitter to the delay (±20%)
        let jitter_factor = 0.8 + (rand::random::<f64>() * 0.4);
        let delay_ms = (self.crawl_delay as f64 * jitter_factor) as u64;
        
        // Wait for the specified delay
        sleep(Duration::from_millis(delay_ms)).await;
    }
    
    /// Start crawling from a set of seed URLs
    pub async fn crawl(&self, seed_urls: Vec<String>) -> Result<(), Box<dyn Error + Send + Sync>> {
        println!("Advanced crawler starting with {} seed URLs", seed_urls.len());
        
        // Initialize URL queue with seed URLs at depth 0
        let url_queue = Arc::new(Mutex::new(
            seed_urls.into_iter().map(|url| (url, 0)).collect::<VecDeque<_>>()
        ));
        
        // Track total pages crawled
        let crawled_count = Arc::new(Mutex::new(0));
        
        // Create worker tasks
        let workers = stream::iter(0..100) // Create up to 100 potential workers
            .map(|id| {
                let client = self.client.clone();
                let url_queue = Arc::clone(&url_queue);
                let visited_urls = Arc::clone(&self.visited_urls);
                let concurrency_limiter = Arc::clone(&self.concurrency_limiter);
                let doc_sender = self.doc_sender.clone();
                let crawled_count = Arc::clone(&crawled_count);
                let robots_cache = Arc::clone(&self.robots_cache);
                let domain_limiters = Arc::clone(&self.domain_rate_limiters);
                let allowed_domains = self.allowed_domains.clone();
                let max_pages = self.max_pages;
                let max_depth = self.max_depth;
                let user_agent = self.user_agent.clone();
                let respect_noindex = self.respect_noindex;
                let respect_nofollow = self.respect_nofollow;
                
                async move {
                    println!("Worker {} started", id);
                    
                    loop {
                        // Check if we've reached the maximum number of pages
                        {
                            let count = *crawled_count.lock().unwrap();
                            if count >= max_pages {
                                println!("Worker {} stopping: reached max pages", id);
                                break;
                            }
                        }
                        
                        // Get the next URL from the queue
                        let work_item = {
                            let mut queue = url_queue.lock().unwrap();
                            queue.pop_front()
                        };
                        
                        // If there's no more work, exit
                        match work_item {
                            Some((url_str, depth)) => {
                                // Skip if we've already visited this URL
                                {
                                    let visited = visited_urls.lock().unwrap();
                                    if visited.contains(&url_str) {
                                        continue;
                                    }
                                }
                                
                                // Parse the URL
                                let url = match Url::parse(&url_str) {
                                    Ok(url) => url,
                                    Err(e) => {
                                        eprintln!("Failed to parse URL '{}': {}", url_str, e);
                                        continue;
                                    }
                                };
                                
                                // Check if domain is allowed
                                if let Some(host) = url.host_str() {
                                    if let Some(ref allowed) = allowed_domains {
                                        if !allowed.contains(host) {
                                            continue;
                                        }
                                    }
                                    
                                    // Acquire a permit from the concurrency limiter
                                    let permit = concurrency_limiter.acquire().await.unwrap();
                                    
                                    // Mark as visited before processing to prevent duplicates
                                    {
                                        let mut visited = visited_urls.lock().unwrap();
                                        visited.insert(url_str.clone());
                                    }
                                    
                                    // Check robots.txt
                                    if !Self::check_robots_allowed(&client, &url, &robots_cache, &user_agent).await {
                                        drop(permit);
                                        continue;
                                    }
                                    
                                    // Respect rate limits for this domain
                                    Self::enforce_rate_limit(host, domain_limiters.clone()).await;
                                    
                                    // Crawl the URL
                                    match Self::process_url(
                                        &client,
                                        &url,
                                        depth,
                                        max_depth,
                                        respect_noindex,
                                        respect_nofollow,
                                        url_queue.clone(),
                                        visited_urls.clone(),
                                    ).await {
                                        Ok(Some(doc)) => {
                                            // Send the document to be indexed
                                            let _ = doc_sender.send(doc).await;
                                            
                                            // Update crawled count
                                            {
                                                let mut count = crawled_count.lock().unwrap();
                                                *count += 1;
                                                
                                                // Print progress periodically
                                                if *count % 10 == 0 {
                                                    println!("Crawled {} pages", *count);
                                                }
                                            }
                                        }
                                        Ok(None) => {
                                            // Page skipped (e.g., not HTML)
                                        }
                                        Err(e) => {
                                            eprintln!("Error processing {}: {}", url, e);
                                        }
                                    }
                                    
                                    // Release the permit
                                    drop(permit);
                                }
                            }
                            None => {
                                // No more URLs in queue, check if all workers are idle
                                let queue_is_empty = url_queue.lock().unwrap().is_empty();
                                if queue_is_empty {
                                    // Sleep briefly to give other workers a chance to add more URLs
                                    sleep(Duration::from_millis(100)).await;
                                    
                                    // Check again
                                    let queue_still_empty = url_queue.lock().unwrap().is_empty();
                                    if queue_still_empty {
                                        println!("Worker {} stopping: queue empty", id);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    
                    println!("Worker {} finished", id);
                }
            })
            .collect::<Vec<_>>();
            
        // Execute all workers concurrently
        futures::future::join_all(workers).await;
        
        let total_crawled = *crawled_count.lock().unwrap();
        println!("Crawler finished. Total pages crawled: {}", total_crawled);
        
        Ok(())
    }
    
    /// Check if a URL is allowed by robots.txt
    async fn check_robots_allowed(
        client: &Client,
        url: &Url,
        robots_cache: &Arc<Mutex<HashMap<String, (ParsedRobots, u64)>>>,
        user_agent: &str,
    ) -> bool {
        if let Some(host) = url.host_str() {
            // Construct robots.txt URL
            let robots_url = format!("{}://{}/robots.txt", url.scheme(), host);
            
            // Try to get from cache
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
                
            let cached_robots = {
                let cache = robots_cache.lock().unwrap();
                cache.get(host).cloned()
            };
            
            let robots = match cached_robots {
                Some((robots, timestamp)) if now - timestamp < 24 * 3600 => {
                    // Cache is still valid
                    robots
                }
                _ => {
                    // Fetch robots.txt
                    match client.get(&robots_url).send().await {
                        Ok(response) => {
                            let content = match response.text().await {
                                Ok(text) => text,
                                Err(_) => String::new(),
                            };
                            
                            let parser = RobotFileParser::new(&robots_url);
                            let robots = parser.parse(&content.lines().collect::<Vec<_>>()).clone();
                            
                            // Update cache
                            {
                                let mut cache = robots_cache.lock().unwrap();
                                cache.insert(host.to_string(), (robots.clone(), now));
                            }
                            
                            robots
                        }
                        Err(_) => {
                            // Failed to fetch robots.txt, assume everything is allowed
                            let parser = RobotFileParser::new(&robots_url);
                            let robots = parser.parse(&[]).clone();
                            
                            // Update cache
                            {
                                let mut cache = robots_cache.lock().unwrap();
                                cache.insert(host.to_string(), (robots.clone(), now));
                            }
                            
                            robots
                        }
                    }
                }
            };
            
            robots.can_fetch(user_agent, url.path())
        } else {
            // No host, can't check robots.txt
            true
        }
    }
    
    /// Enforce rate limiting for a domain
    async fn enforce_rate_limit(domain: &str, limiters: Arc<Mutex<HashMap<String, tokio::sync::Mutex<()>>>>) {
        // Get or create a mutex for this domain
        let rate_limiter = {
            let mut limiters_map = limiters.lock().unwrap();
            if !limiters_map.contains_key(domain) {
                limiters_map.insert(domain.to_string(), tokio::sync::Mutex::new(()));
            }
            limiters_map.get(domain).unwrap().clone()
        };
        
        // Acquire the lock
        let _guard = rate_limiter.lock().await;
        
        // Add randomized delay (between 500ms and 2000ms)
        let delay = rand::thread_rng().gen_range(500..2000);
        sleep(Duration::from_millis(delay)).await;
    }
    
    /// Process a URL, extract content and links
    async fn process_url(
        client: &Client,
        url: &Url,
        depth: u32,
        max_depth: u32,
        respect_noindex: bool,
        respect_nofollow: bool,
        url_queue: Arc<Mutex<VecDeque<(String, u32)>>>,
        visited_urls: Arc<Mutex<HashSet<String>>>,
    ) -> Result<Option<CrawledDocument>, Box<dyn Error + Send + Sync>> {
        // Fetch the page
        let response = match client.get(url.clone()).send().await {
            Ok(resp) => resp,
            Err(e) => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))),
        };
        
        if !response.status().is_success() {
            return Ok(None);
        }
        
        // Check content type
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
            
        if !content_type.contains("text/html") {
            return Ok(None);
        }
        
        // Read the content
        let html = match response.text().await {
            Ok(text) => text,
            Err(e) => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))),
        };
        
        // Parse the HTML
        let document = Html::parse_document(&html);
        
        // Check for noindex meta tag
        if respect_noindex {
            let meta_selector = Selector::parse(r#"meta[name="robots"], meta[name="googlebot"]"#).unwrap();
            for meta in document.select(&meta_selector) {
                if let Some(content) = meta.value().attr("content") {
                    if content.contains("noindex") {
                        return Ok(None);
                    }
                }
            }
        }
        
        // Extract title
        let title_selector = Selector::parse("title").unwrap();
        let title = document
            .select(&title_selector)
            .next()
            .map(|element| element.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_else(|| url.to_string());
            
        // Extract main content
        let body_selector = Selector::parse("body").unwrap();
        let text = document
            .select(&body_selector)
            .next()
            .map(|element| {
                // Clean text: remove scripts, styles, etc.
                let script_selector = Selector::parse("script, style, noscript, iframe, object, embed").unwrap();
                let mut content_html = element.html();
                for node in document.select(&script_selector) {
                    let node_html = node.html();
                    content_html = content_html.replace(&node_html, "");
                }
                
                // Parse again and get text
                let clean_fragment = Html::parse_fragment(&content_html);
                clean_fragment.root_element().text().collect::<Vec<_>>().join(" ")
            })
            .unwrap_or_default();
            
        // Extract links if we're below max depth
        if depth < max_depth {
            let link_selector = Selector::parse("a[href]").unwrap();
            
            for link in document.select(&link_selector) {
                if respect_nofollow {
                    // Skip nofollow links
                    if link.value().attr("rel").map_or(false, |rel| rel.contains("nofollow")) {
                        continue;
                    }
                }
                
                if let Some(href) = link.value().attr("href") {
                    // Resolve the URL
                    if let Ok(resolved_url) = url.join(href) {
                        // Normalize the URL
                        let normalized = Self::normalize_url(&resolved_url);
                        
                        // Add to queue if not visited
                        let visited = visited_urls.lock().unwrap();
                        if !visited.contains(&normalized) {
                            let mut queue = url_queue.lock().unwrap();
                            queue.push_back((normalized, depth + 1));
                        }
                    }
                }
            }
        }
        
        // Return the crawled document
        Ok(Some(CrawledDocument {
            url: url.to_string(),
            title,
            text,
        }))
    }
    
    /// Normalize a URL for consistency
    fn normalize_url(url: &Url) -> String {
        let mut normalized = url.clone();
        
        // Remove fragments
        normalized.set_fragment(None);
        
        // Remove default ports
        if (url.scheme() == "http" && url.port() == Some(80)) || 
           (url.scheme() == "https" && url.port() == Some(443)) {
            normalized.set_port(None).unwrap_or(());
        }
        
        // Ensure trailing slash on domain roots
        if normalized.path() == "" {
            normalized.set_path("/");
        }
        
        normalized.to_string()
    }
}

/// Simple error type for crawler operations
#[derive(Debug)]
pub struct CrawlerError {
    message: String,
}

impl fmt::Display for CrawlerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Crawler error: {}", self.message)
    }
}

impl Error for CrawlerError {}

// Make CrawlerError Send and Sync
unsafe impl Send for CrawlerError {}
unsafe impl Sync for CrawlerError {}