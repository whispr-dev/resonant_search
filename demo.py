from engine import ResonantEngine

engine = ResonantEngine()
engine.load_directory("data")  # Auto-load corpus

query = input("Enter your resonant query: ")
results = engine.search(query)

print("\nTop Resonant Matches:")
for idx, r in enumerate(results):
    print(f"[{idx+1}] {r['title']}")
    print(f"    Resonance:      {r['resonance']:.4f}")
    print(f"    Δ Entropy:      {r['delta_entropy']:.4f}")
    print(f"    Combined Score: {r['score']:.4f}")

