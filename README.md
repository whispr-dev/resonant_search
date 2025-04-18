# resonant_search
 resonance based search engine
File: README.md

# Resonant Search Engine

A symbolic, entropy-aware search engine that finds meaning by **resonance**, not just keywords.

Instead of matching literal words, this engine measures **conceptual alignment** between queries and documents using:

- Prime-based vector encoding (symbolic fingerprinting)
- Shannon entropy (semantic "chaos" vs "coherence")
- Phase resonance (alignment of ideas)

It ranks documents based on how well their symbolic patterns **resonate with your query**, adjusted for how structurally similar they are.

---

## How It Works

1. **Token-to-Prime Mapping**
   Every word becomes a unique prime number — your document becomes a vector of primes.
   Think: "symbolic coordinates" for meaning.

2. **Hilbert Vector Construction**
   We build a normalized vector for each document, where word frequency forms the structure.

3. **Resonance Scoring**
   A dot product compares your query's symbolic vector to each document’s — stronger overlap = stronger resonance.

4. **Entropy Difference (ΔS)**
   Each document’s word distribution is scored for entropy (disorder). A perfect match in structure lowers the entropy penalty.

5. **Final Score**

final_score = resonance - delta_entropy × weight

---

## Example

Query:  
```text
entropy resonance symmetry collapse
Result:

text
[1] collapse_and_decay
    Resonance:      0.4472
    Δ Entropy:      1.7500
    Combined Score: 0.2722
    Preview:        Entropy drives collapse. Resonance emerges in the balance of order and chaos...

Try It Out
Put your documents in /data/:
Each .txt file becomes a searchable document.

bash
resonant_search/
├── data/
│   ├── entropy_and_form.txt
│   ├── symbols_of_order.txt
│   └── collapse_and_decay.txt

Run the search:
bash
Copy
Edit
python demo.py
Enter your idea as a query:

text
Enter your resonant query: entropy symmetry collapse
You’ll get top matches ranked by conceptual resonance and structural similarity.

Why Prime Numbers?
Primes are indivisible, fundamental — like atoms of meaning. By encoding symbols into a prime-based Hilbert space, we get a lightweight stand-in for a kind of quantum-like semantic system.

This engine is inspired by metaphysical math ideas like:
- Symbolic entropy and resonance collapse
- Prime-based number fields
- Phase coherence in concept space

But don’t worry — you don’t need math to use it. Just ideas.

Tech Stack
Python 3
sympy (for prime numbers)
numpy (vector ops)

Pure text and math — no AI required (yet)

What’s Next?
-  Add fuzzy keyword highlighting
- Visualize symbolic collapse
- Save token-prime mapping across sessions
- Build a GUI or terminal dashboard
- LLM hybrid mode (ask GPT to explain top results)

Built By
A curious fren exploring how chaos collapses into meaning.
This is a tool for artists, hackers, thinkers — anyone looking for resonance in the noise.

“The universe hums with symbols — this engine helps you tune in.”

---