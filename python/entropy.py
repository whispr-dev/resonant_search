import math
from collections import Counter

def shannon_entropy(primes):
    """
    Calculate Shannon entropy of a list of prime tokens.
    """
    if not primes:
        return 0.0
    counts = Counter(primes)
    total = len(primes)
    entropy = 0.0
    for count in counts.values():
        p = count / total
        entropy -= p * math.log2(p)
    return entropy
