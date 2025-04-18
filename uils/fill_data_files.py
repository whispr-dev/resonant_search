import os
import random
import zipfile

# Word lists for generating poetic sentences
subjects = [
    "Entropy", "Collapse", "Resonance", "Chaos", "Order", "Symmetry", "Form",
    "Structure", "Void", "Cycle", "Force", "Balance", "Fractal", "Pattern", "Flux"
]
verbs = [
    "is", "reveals", "binds", "shapes", "hides", "weaves", "unfolds", "drives",
    "mirrors", "breaks"
]
objects = [
    "hidden order", "symmetry", "truth", "chaos", "balance", "essence", "void",
    "origin", "pattern", "eternal cycle", "fractal dance", "cosmic flux", "form",
    "resonant truth", "structure"
]

# Generate a unique poetic sentence
def generate_poetic_sentence():
    subject = random.choice(subjects)
    verb = random.choice(verbs)
    obj = random.choice(objects)
    # Ensure variety by avoiding repetitive combos
    while subject.lower() in obj.lower() or verb.lower() in obj.lower():
        obj = random.choice(objects)
    return f"{subject} {verb} the {obj}."

# Ensure data directory exists
output_dir = "data"
if not os.path.exists(output_dir):
    raise FileNotFoundError("Fren, the 'data/' folder is missing! Run the previous script to create it.")

# Get list of .txt files in data/
txt_files = [f for f in os.listdir(output_dir) if f.endswith(".txt")]
if len(txt_files) != 50:
    print(f"Warning, Fren: Found {len(txt_files)} .txt files, expected 50. Proceeding anyway.")

# Generate unique sentences
sentences = set()
while len(sentences) < len(txt_files):
    sentences.add(generate_poetic_sentence())
sentences = list(sentences)

# Write unique sentences to each .txt file
for file_name, sentence in zip(txt_files, sentences):
    file_path = os.path.join(output_dir, file_name)
    with open(file_path, "w") as f:
        f.write(sentence)

# Create a zip file
zip_name = "data_archive.zip"
with zipfile.ZipFile(zip_name, "w", zipfile.ZIP_DEFLATED) as zipf:
    for file_name in txt_files:
        file_path = os.path.join(output_dir, file_name)
        zipf.write(file_path, os.path.join(output_dir, file_name))

print(f"Fren, your {zip_name} is ready with {len(txt_files)} files in {output_dir}/, each with a unique poetic line!")