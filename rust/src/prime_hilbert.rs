// src/prime_hilbert.rs - Complete with missing functions

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use num_complex::Complex;

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

/// Calculates a complex resonance between two vectors with decay factor
pub fn resonance_complex(vec1: &PrimeVector, vec2: &PrimeVector, decay_factor: f64) -> Complex<f64> {
    let real_part = dot_product(vec1, vec2);
    
    // Calculate a phase component based on the prime distribution
    let mut phase = 0.0;
    for (prime, freq1) in vec1 {
        if let Some(freq2) = vec2.get(prime) {
            // Use the prime number itself to contribute to phase
            phase += (*prime as f64).ln() * freq1 * freq2;
        }
    }
    
    // Apply decay factor
    let decayed_real = real_part * (-decay_factor).exp();
    let decayed_imag = phase * (-decay_factor * 0.5).exp(); // Slower decay for imaginary part
    
    Complex::new(decayed_real, decayed_imag)
}

/// Converts a sparse PrimeVector to a dense Vec<f64> of a specified max dimension.
pub fn to_dense_vector(sparse_vec: &PrimeVector, max_prime_value: u64) -> Vec<f64> {
    let mut dense_vec = vec![0.0; (max_prime_value + 1) as usize];
    for (&prime, &freq) in sparse_vec {
        if (prime as usize) < dense_vec.len() {
            dense_vec[prime as usize] = freq;
        }
    }
    dense_vec
}

/// Calculates a score based on the query's resonance with the biorthogonal components of a document.
pub fn biorthogonal_score(query_bio: &BiorthogonalVector, doc_bio: &BiorthogonalVector) -> f64 {
    // Calculate dot products with both left and right components
    let score_left = dot_product(&query_bio.left, &doc_bio.left);
    let score_right = dot_product(&query_bio.right, &doc_bio.right);
    // Combine them
    score_left + score_right
}