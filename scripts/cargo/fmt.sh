#!/bin/bash

# Save the original rustfmt.toml content
original_content="hard_tabs = true"

# New content to be added. These are all unstable, and require nightly to run.
new_content="
# https://rust-lang.github.io/rustfmt/?version=v1.6.0&search=import#Crate%5C%3A
imports_granularity = \"Crate\"

# https://rust-lang.github.io/rustfmt/?version=v1.6.0&search=import#StdExternalCrate%5C%3A
group_imports = \"StdExternalCrate\""

# Switch to nightly toolchain
rustup default nightly

# Modify the rustfmt.toml file in the root directory
echo "$original_content$new_content" > rustfmt.toml

# Find all Cargo.toml files in subdirectories
for cargo_toml in $(find . -name "Cargo.toml"); do
    # Ignore auto-generated files
    if [[ $cargo_toml == *"gen"* ]] || [[ $cargo_toml == *"lib/smithy-output"* ]] || [[ $cargo_toml == *"sdks"* ]]; then
        continue
    fi

    # Run rustfmt with the --manifest-path option in the background
    echo "Formatting $cargo_toml"
    cargo fmt --manifest-path $cargo_toml &
done

# Wait for all background jobs to finish
wait

# Revert the rustfmt.toml file in the root directory
echo "$original_content" > rustfmt.toml

# Switch back to stable toolchain
rustup default stable