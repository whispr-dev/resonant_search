import os
from tokenizer import PrimeTokenizer
from prime_hilbert import build_vector, dot_product
from entropy import shannon_entropy

class ResonantEngine:
    def __init__(self):
        self.tokenizer = PrimeTokenizer()
        self.docs = []

    def add_document(self, title, text):
        tokens = self.tokenizer.tokenize(text)
        vec = build_vector(tokens)
        entropy = shannon_entropy(tokens)
        self.docs.append({
            "title": title,
            "text": text,
            "vector": vec,
            "entropy": entropy
        })

    def load_directory(self, folder):
        """
        Load and index all .txt files in a directory.
        """
        for fname in os.listdir(folder):
            if fname.endswith(".txt"):
                path = os.path.join(folder, fname)
                with open(path, "r", encoding="utf-8") as f:
                    text = f.read()
                    title = os.path.splitext(fname)[0]
                    self.add_document(title, text)

    def search(self, query, top_k=3):
        query_tokens = self.tokenizer.tokenize(query)
        query_vec = build_vector(query_tokens)
        query_entropy = shannon_entropy(query_tokens)

        results = []
        for doc in self.docs:
            resonance = dot_product(query_vec, doc["vector"])
            delta_entropy = abs(doc["entropy"] - query_entropy)
            score = resonance - delta_entropy * 0.1
            results.append({
                "title": doc["title"],
                "resonance": resonance,
                "delta_entropy": delta_entropy,
                "score": score,
                "snippet": doc["text"][:200].strip().replace('\n', ' ') + '...'  # clean + trim preview
            })

        results.sort(key=lambda x: x["score"], reverse=True)
        return results[:top_k]
