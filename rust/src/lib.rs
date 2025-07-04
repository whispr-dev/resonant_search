// src/lib.rs

pub mod tokenizer;
pub mod entropy;
pub mod prime_hilbert;
pub mod engine;
pub mod crawler;
pub mod quantum_types;

// Re-export key types and functions
pub use engine::ResonantEngine;
pub use engine::SearchResult;
pub use crawler::CrawledDocument;
pub use prime_hilbert::{PrimeVector, BiorthogonalVector};
pub use quantum_types::{MatrixComplex, VectorComplex};

// Export key persistence theory functions
pub use entropy::{
    shannon_entropy,
    calculate_reversibility,
    entropy_pressure,
    buffering_capacity,
    persistence_score
};

// Export quantum functions from their proper module
pub use quantum_types::{
    mutual_information,
    calculate_redundancy,
    calculate_symmetry
};