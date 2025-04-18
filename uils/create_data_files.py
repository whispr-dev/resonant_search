import os
import random
import zipfile

# Define word lists for generating thematic file names
prefixes = [
    "entropy", "symbols", "binding", "collapse", "order", "chaos", "form", "structure",
    "resonance", "decay", "pattern", "flux", "void", "cycle", "force", "balance",
    "fractal", "symmetry", "tension", "emergence"
]
suffixes = [
    "form", "order", "collapse", "structure", "flow", "essence", "void", "cycle",
    "pattern", "force", "balance", "decay", "resonance", "chaos", "origin", "state",
    "field", "motion", "link", "shift"
]

# Ensure unique file names
def generate_file_names(n=50):
    file_names = set()
    while len(file_names) < n:
        prefix = random.choice(prefixes)
        suffix = random.choice(suffixes)
        if prefix != suffix:  # Avoid same word combos
            file_name = f"{prefix}_and_{suffix}.txt"
            file_names.add(file_name)
    return list(file_names)

# Create data directory and files
output_dir = "data"
os.makedirs(output_dir, exist_ok=True)

# Generate 50 unique file names
file_names = generate_file_names(50)

# Create text files with placeholder content
for file_name in file_names:
    file_path = os.path.join(output_dir, file_name)
    with open(file_path, "w") as f:
        f.write(f"This is {file_name}, ready for your data!")

# Create a zip file
zip_name = "data_archive.zip"
with zipfile.ZipFile(zip_name, "w", zipfile.ZIP_DEFLATED) as zipf:
    for file_name in file_names:
        file_path = os.path.join(output_dir, file_name)
        zipf.write(file_path, os.path.join(output_dir, file_name))

print(f"Fren, your {zip_name} is ready with 50 files in {output_dir}/!")