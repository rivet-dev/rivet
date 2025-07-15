#!/usr/bin/env node

import { execSync } from 'child_process';
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join } from 'path';

const REPO_URL = 'https://github.com/rivet-gg/rivetkit.git';
const TEMP_DIR = '/tmp/rivetkit-examples';
const OUTPUT_DIR = './src/data/examples';

// Ensure output directory exists
if (!existsSync(OUTPUT_DIR)) {
  mkdirSync(OUTPUT_DIR, { recursive: true });
}

// Clone or update the repository
function updateRepo() {
  if (existsSync(TEMP_DIR)) {
    console.log('Updating existing repository...');
    execSync('git clean -fd', { cwd: TEMP_DIR });
    execSync('git reset --hard', { cwd: TEMP_DIR });
    execSync('git fetch origin', { cwd: TEMP_DIR });
    execSync('git checkout 07-09-chore_add_new_examples', { cwd: TEMP_DIR });
    execSync('git pull origin 07-09-chore_add_new_examples', { cwd: TEMP_DIR });
  } else {
    console.log('Cloning repository...');
    execSync(`git clone -b 07-09-chore_add_new_examples ${REPO_URL} ${TEMP_DIR}`);
  }
}

// Replace workspace dependencies with version numbers
function replaceWorkspaceDependencies(content) {
  return content.replace(/@rivetkit\/([^"]+)": "workspace:\*"/g, '@rivetkit/$1": "^0.9.1"');
}

// Get all available examples from the repository
function getAvailableExamples() {
  const examplesDir = join(TEMP_DIR, 'examples');
  
  if (!existsSync(examplesDir)) {
    console.warn('Examples directory not found');
    return [];
  }
  
  try {
    const dirs = execSync('find . -maxdepth 1 -type d', { 
      cwd: examplesDir,
      encoding: 'utf-8'
    }).trim().split('\n')
      .map(dir => dir.replace('./', ''))
      .filter(dir => dir !== '.' && dir !== '' && dir !== 'snippets');
    
    console.log(`Found ${dirs.length} examples: ${dirs.join(', ')}`);
    return dirs;
  } catch (error) {
    console.error('Error discovering examples:', error.message);
    return [];
  }
}

// Read and process ALL files from an example directory
function processExample(exampleName) {
  const exampleDir = join(TEMP_DIR, 'examples', exampleName);
  
  if (!existsSync(exampleDir)) {
    console.warn(`Example directory not found: ${exampleName}`);
    return null;
  }

  const files = {};
  
  try {
    // Get all files recursively in the directory, including subdirectories
    const allFiles = execSync('find . -type f -not -path "*/node_modules/*" -not -path "*/.git/*" -not -path "*/dist/*" -not -path "*/build/*" -not -name ".*"', { 
      cwd: exampleDir,
      encoding: 'utf-8'
    }).trim().split('\n');
    
    for (const file of allFiles) {
      if (file && file !== '.') {
        const filePath = join(exampleDir, file);
        const relativePath = file.replace('./', '');
        
        if (existsSync(filePath)) {
          try {
            let content = readFileSync(filePath, 'utf-8');
            
            // Replace workspace dependencies in package.json files
            if (relativePath.endsWith('package.json')) {
              content = replaceWorkspaceDependencies(content);
            }
            
            files[relativePath] = content;
          } catch (readError) {
            // Skip binary files or files that can't be read as text
            console.warn(`Skipping file ${relativePath}: ${readError.message}`);
          }
        }
      }
    }
    
    console.log(`Found ${Object.keys(files).length} files in ${exampleName}`);
    return files;
  } catch (error) {
    console.warn(`Error reading files from ${exampleName}:`, error.message);
    return null;
  }
}

// Main function
function main() {
  console.log('Generating examples...');
  
  // Update the repository
  updateRepo();
  
  // Discover available examples from the repository
  const examples = getAvailableExamples();
  
  if (examples.length === 0) {
    console.error('No examples found in the repository');
    return;
  }
  
  const generatedExamples = {};
  
  // Process each example
  for (const exampleName of examples) {
    console.log(`Processing example: ${exampleName}`);
    const files = processExample(exampleName);
    
    if (files && Object.keys(files).length > 0) {
      generatedExamples[exampleName] = files;
      console.log(`✓ Processed ${exampleName} with ${Object.keys(files).length} files`);
    } else {
      console.warn(`⚠ No files found for example: ${exampleName}`);
    }
  }
  
  // Write the generated examples to a JSON file
  const outputPath = join(OUTPUT_DIR, 'examples.json');
  writeFileSync(outputPath, JSON.stringify(generatedExamples, null, 2));
  
  console.log(`✓ Generated examples.json with ${Object.keys(generatedExamples).length} examples`);
  console.log('Examples generation complete!');
}

main();
