// src/prime_hilbert.rs - Corrected biorthogonal_score signature and implementation

use std::collections::HashMap;
use serde::{Serialize, Deserialize}; // Needed for BiorthogonalVector serialization

// Define PrimeVector as a type alias for HashMap
pub type PrimeVector = HashMap<u64, f64>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiorthogonalVector {
    pub left: HashMap<u64, f64>,
    pub right: HashMap<u64, f64>,
}

/// Builds a PrimeVector (frequency map) from a list of prime tokens.
pub fn build_vector(tokens: &[u64]) -> PrimeVector {
    let mut vec = HashMap::new();
    for &token in tokens {
        *vec.entry(token).or_insert(0.0) += 1.0;
    }
    // Normalize to frequency
    let total_tokens = tokens.len() as f64;
    if total_tokens > 0.0 {
        for (_prime, freq) in vec.iter_mut() {
            *freq /= total_tokens;
        }
    }
    vec
}

/// Computes the dot product of two PrimeVectors (sparse representations).
pub fn dot_product(vec1: &PrimeVector, vec2: &PrimeVector) -> f64 {
    let mut sum = 0.0;
    for (prime, freq1) in vec1 {
        if let Some(freq2) = vec2.get(prime) {
            sum += freq1 * freq2;
        }
    }
    sum
}

/// Builds a biorthogonal vector (two components) from a list of prime tokens.
// Corrected to take &[u64] as input
pub fn build_biorthogonal_vector(primes: &[u64]) -> BiorthogonalVector {
    let mut left = HashMap::new();
    let mut right = HashMap::new();

    for &prime in primes {
        // Simple example: distribute token frequency to left and right components
        // In a real system, this would be based on some deeper mathematical property
        *left.entry(prime).or_insert(0.0) += 0.5;
        *right.entry(prime).or_insert(0.0) += 0.5;
    }

    // Normalize (optional, depending on intended use)
    let total_primes = primes.len() as f64;
    if total_primes > 0.0 {
        for (_prime, freq) in left.iter_mut() {
            *freq /= total_primes;
        }
        for (_prime, freq) in right.iter_mut() {
            *freq /= total_primes;
        }
    }

    BiorthogonalVector { left, right }
}

/// Converts a sparse PrimeVector to a dense Vec<f64> of a specified max dimension.
// Used for historical vectors, requires a max_prime_value to determine vector length
pub fn to_dense_vector(sparse_vec: &PrimeVector, max_prime_value: u64) -> Vec<f64> {
    let mut dense_vec = vec![0.0; (max_prime_value + 1) as usize]; // +1 for 0-based indexing up to max_prime_value
    for (&prime, &freq) in sparse_vec {
        if (prime as usize) < dense_vec.len() {
            dense_vec[prime as usize] = freq;
        }
    }
    dense_vec
}

/// Calculates a score based on the query's resonance with the biorthogonal components of a document.
// FIXED: Corrected signature to take PrimeVector for query
pub fn biorthogonal_score(query: &PrimeVector, doc_biorthogonal: &BiorthogonalVector) -> f64 {
    // Calculate dot products with both left and right components of the biorthogonal vector
    let score_left = dot_product(query, &doc_biorthogonal.left);
    let score_right = dot_product(query, &doc_biorthogonal.right);
    // Combine them, perhaps average or sum as appropriate for your model
    score_left + score_right
}