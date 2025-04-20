import numpy as np

def build_vector(primes):
    """
    Takes a list of prime tokens and builds a normalized frequency vector.
    """
    vector = {}
    for p in primes:
        vector[p] = vector.get(p, 0) + 1
    norm = np.sqrt(sum(v ** 2 for v in vector.values()))
    for k in vector:
        vector[k] /= norm
    return vector

def dot_product(vec1, vec2):
    """
    Sparse dot product for prime-based vectors.
    """
    return sum(vec1.get(k, 0) * vec2.get(k, 0) for k in set(vec1) | set(vec2))
