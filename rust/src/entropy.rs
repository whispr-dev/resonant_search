// src/entropy.rs

use std::collections::HashMap;
use crate::quantum_types::{mutual_information, calculate_redundancy, calculate_symmetry};

/// Calculates the Shannon entropy of a list of u64 values (prime tokens).
pub fn shannon_entropy(primes: &[u64]) -> f64 {
    if primes.is_empty() {
        return 0.0;
    }

    // Count the occurrences of each prime
    let mut counts = HashMap::new();
    for &prime in primes {
        *counts.entry(prime).or_insert(0) += 1;
    }

    let total_count = primes.len() as f64;
    let mut entropy = 0.0;

    // Calculate entropy using the formula: -sum(p * log2(p))
    for &count in counts.values() {
        let p = count as f64 / total_count;
        entropy -= p * f64::log2(p);
    }

    entropy
}

/// Calculate the reversibility between a document vector and historical vectors
pub fn calculate_reversibility(doc_vector: &Vec<f64>, historical_vectors: &[Vec<f64>]) -> f64 {
    if historical_vectors.is_empty() {
        return 1.0; // By default, a vector is fully reversible with itself
    }
    
    historical_vectors.iter()
        .map(|past_vec| mutual_information(doc_vector, past_vec))
        .sum::<f64>() / historical_vectors.len() as f64
}

/// Calculate entropy pressure based on document age and frequency metrics
pub fn entropy_pressure(doc_age: f64, update_frequency: f64, trend_decay: f64) -> f64 {
    update_frequency * trend_decay * doc_age.exp()
}

/// Calculate the buffering capacity of a document vector
pub fn buffering_capacity(doc_vector: &Vec<f64>) -> f64 {
    let redundancy = calculate_redundancy(doc_vector);
    let symmetry = calculate_symmetry(doc_vector);
    redundancy + symmetry
}

/// Calculate a persistence score based on thermodynamic parameters
pub fn persistence_score(
    reversibility: f64, 
    entropy_pressure: f64, 
    buffering: f64, 
    fragility: f64
) -> f64 {
    if buffering <= 0.0 {
        return 0.0; // Avoid division by zero
    }
    ((-fragility) * (1.0 - reversibility) * (entropy_pressure / buffering)).exp()
}