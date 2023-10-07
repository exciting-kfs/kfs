import os
import re

# Define the project directory
project_directory = './'

# Regular expression pattern to match 'trace_feature!("feat-name", .*)' with optional additional arguments
pattern = r'trace_feature!\s*\(\s*("[^"]+").*\)'

# Function to search for and extract feature names
def extract_feature_names(file_path):
    feature_names = []
    with open(file_path, 'r') as file:
        content = file.read()
        matches = re.findall(pattern, content)
        feature_names.extend(matches)
    return feature_names

# Function to search for 'trace_feature!("feat-name", .*)' in Rust files
def search_for_trace_feature(directory):
    feature_names = []
    for root, _, files in os.walk(directory):
        for file in files:
            if file.endswith('.rs'):
                file_path = os.path.join(root, file)
                feature_names.extend(extract_feature_names(file_path))
    return feature_names

# Call the function to search for 'trace_feature!("feat-name", .*)' in Rust files and print the extracted feature names
feature_names = search_for_trace_feature(project_directory)
unique_feature_names = set(feature_names)

print('[Trace Feature List]:')
for name in unique_feature_names:
    print(name.strip('"'))
