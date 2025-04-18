import re
from sympy import primerange, nextprime

class PrimeTokenizer:
    def __init__(self):
        self.token_to_prime = {}
        self.prime_to_token = {}
        self.current_prime = 2

    def tokenize(self, text):
        tokens = re.findall(r'\b\w+\b', text.lower())
        primes = []
        for token in tokens:
            if token not in self.token_to_prime:
                self.token_to_prime[token] = self.current_prime
                self.prime_to_token[self.current_prime] = token
                self.current_prime = nextprime(self.current_prime)
            primes.append(self.token_to_prime[token])
        return primes

    def print_vocab(self):
        for token, prime in self.token_to_prime.items():
            print(f"{token}: {prime}")
