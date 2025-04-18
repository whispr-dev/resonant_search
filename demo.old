from tokenizer import PrimeTokenizer
from prime_hilbert import build_vector, dot_product
from entropy import shannon_entropy

t = PrimeTokenizer()

doc1 = "entropy collapse resonance"
doc2 = "symbolic entropy binds meaning"
doc3 = "prime numbers reveal order in chaos"

# Tokenize and vectorize
vec1 = build_vector(t.tokenize(doc1))
vec2 = build_vector(t.tokenize(doc2))
vec3 = build_vector(t.tokenize(doc3))

# Compare all vs all
print("resonance(doc1, doc2):", dot_product(vec1, vec2))
print("resonance(doc1, doc3):", dot_product(vec1, vec3))
print("resonance(doc2, doc3):", dot_product(vec2, vec3))

# Show token mapping
print("\nToken mapping:")
t.print_vocab()

print("\nEntropy:")
print("doc1 entropy:", shannon_entropy(t.tokenize(doc1)))
print("doc2 entropy:", shannon_entropy(t.tokenize(doc2)))
print("doc3 entropy:", shannon_entropy(t.tokenize(doc3)))
