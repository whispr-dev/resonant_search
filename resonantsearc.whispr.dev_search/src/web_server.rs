// src/web_server.rs

use crate::engine::{ResonantEngine, SearchResult};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

// Define the shared state for our web server
pub struct AppState {
    pub engine: Arc<Mutex<ResonantEngine>>,
}

// Input query struct
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    10
}

// Search result response struct
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    query: String,
    results: Vec<SearchResultResponse>,
    elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct SearchResultResponse {
    title: String,
    url: String,
    snippet: String,
    score: f64,
    quantum_score: Option<f64>,
    persistence_score: Option<f64>,
}

// Initialize and start the web server
pub async fn start_server(
    engine: ResonantEngine,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Wrap the engine in Arc and Mutex for thread safety
    let shared_state = Arc::new(AppState {
        engine: Arc::new(Mutex::new(engine)),
    });

    // Build our router
    let app = Router::new()
        // API routes
        .route("/api/search", get(search_handler))
        .route("/api/health", get(health_handler))
        
        // Web interface routes
        .route("/", get(index_handler))
        
        // Static file serving
        .nest_service("/static", ServeDir::new("static"))
        
        // Add tracing
        .layer(TraceLayer::new_for_http())
        
        // Add shared state
        .with_state(shared_state);

    // Start the server
    info!("Starting web server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Handler for the main search API endpoint
async fn search_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    info!("Processing search query: {}", params.q);
    
    if params.q.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(SearchResponse {
                query: params.q,
                results: vec![],
                elapsed_ms: 0,
            }),
        );
    }
    
    let start_time = std::time::Instant::now();
    
    // Acquire lock and perform search
    let results = {
        match state.engine.lock() {
            Ok(mut engine) => {
                // Perform the search
                engine.search(&params.q, params.limit)
            }
            Err(e) => {
                warn!("Failed to acquire lock on engine: {}", e);
                vec![]
            }
        }
    };
    
    // Convert internal results to response format
    let response_results = results
        .into_iter()
        .map(|r| SearchResultResponse {
            title: r.title,
            url: r.path,
            snippet: r.snippet,
            score: r.score,
            quantum_score: if r.quantum_score != 0.0 { Some(r.quantum_score) } else { None },
            persistence_score: if r.persistence_score != 0.0 { Some(r.persistence_score) } else { None },
        })
        .collect();

    let elapsed = start_time.elapsed().as_millis() as u64;
    info!("Search for '{}' completed in {}ms", params.q, elapsed);
    
    // Return the formatted response
    (
        StatusCode::OK,
        Json(SearchResponse {
            query: params.q,
            results: response_results,
            elapsed_ms: elapsed,
        }),
    )
}

// Health check endpoint
async fn health_handler() -> &'static str {
    "OK"
}

// Main landing page handler
async fn index_handler() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}