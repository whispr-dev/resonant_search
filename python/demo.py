from engine import ResonantEngine

# Initialize engine
engine = ResonantEngine()

# Load all .txt files in /data/
engine.load_directory("data")

# Get query
query = input("Enter your resonant query: ")

# Perform search
results = engine.search(query)

# Show results with snippet previews
print("\nTop Resonant Matches:")
for idx, r in enumerate(results):
    print(f"[{idx+1}] {r['title']}")
    print(f"    Resonance:      {r['resonance']:.4f}")
    print(f"    Δ Entropy:      {r['delta_entropy']:.4f}")
    print(f"    Combined Score: {r['score']:.4f}")
    print(f"    Preview:        {r['snippet']}")
    print()
