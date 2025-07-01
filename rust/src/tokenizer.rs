// src/tokenizer.rs

use regex::Regex;
use std::collections::HashMap;

/// A tokenizer that maps words to unique prime numbers.
pub struct PrimeTokenizer {
    token_to_prime: HashMap<String, u64>,
    prime_to_token: HashMap<u64, String>,
    current_prime: u64,
    word_regex: Regex,
}

impl PrimeTokenizer {
    /// Creates a new `PrimeTokenizer`.
    pub fn new() -> Self {
        let word_regex = Regex::new(r"\b\w+\b").expect("Failed to create word regex");
        PrimeTokenizer {
            token_to_prime: HashMap::new(),
            prime_to_token: HashMap::new(),
            current_prime: 2, // Start with the first prime
            word_regex,
        }
    }

    /// Gets the next prime after current_prime
    fn next_prime(&mut self) -> u64 {
        loop {
            self.current_prime += if self.current_prime == 2 { 1 } else { 2 };
            if primal::is_prime(self.current_prime) {
                return self.current_prime;
            }
        }
    }

    /// Tokenizes the input text into a vector of prime numbers.
    pub fn tokenize(&mut self, text: &str) -> Vec<u64> {
        let lower_text = text.to_lowercase();
        let mut primes_list = Vec::new();
        
        // Collect tokens first to avoid borrow checker issues
        let tokens: Vec<String> = self.word_regex
            .find_iter(&lower_text)
            .map(|mat| mat.as_str().to_string())
            .collect();

        for token in tokens {
            if !self.token_to_prime.contains_key(&token) {
                let next_p = self.next_prime();
                self.token_to_prime.insert(token.clone(), next_p);
                self.prime_to_token.insert(next_p, token.clone());
            }
            primes_list.push(*self.token_to_prime.get(&token).unwrap());
        }

        primes_list
    }
    
    /// Tokenizes the input prime numbers without updating the vocabulary.
    /// This is useful when we want to generate tokens without affecting the tokenizer's state.
    pub fn tokenize_without_update(&self, primes: &[u64]) -> Vec<u64> {
        // Simply return the primes as is, since they're already prime tokens
        primes.to_vec()
    }

    #[allow(dead_code)]
    /// Prints the current vocabulary (token to prime mapping).
    pub fn print_vocab(&self) {
        for (token, prime) in &self.token_to_prime {
            println!("{}: {}", token, prime);
        }
    }

    #[allow(dead_code)]
    /// Returns the token associated with a prime number, if it exists.
    pub fn get_token(&self, prime: u64) -> Option<&String> {
        self.prime_to_token.get(&prime)
    }

    #[allow(dead_code)]
    /// Returns the prime number associated with a token, if it exists.
    pub fn get_prime(&self, token: &str) -> Option<&u64> {
        self.token_to_prime.get(token)
    }
}