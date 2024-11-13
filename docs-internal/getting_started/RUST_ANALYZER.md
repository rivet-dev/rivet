# Rust Analyzer

Rivet uses a Nix shell to install some dependencies that our Rust libraries require. This can cause potential
issues with Rust Analyzer, which doesn't use the Nix shell.

## Visual Studio Code & other IDEs

Debian & Ubuntu:

```sh
# Install packages
sudo apt update -y
sudo apt install -y libssl-dev pkg-config protobuf-compiler

# Update Rust, make sure you have rustup (https://rustup.rs/)
rustup update
```

Reload VS Code (or restart the language server).

## Vim & Emacs

TUI-based editors can be ran from within a Nix shell. Make sure you're inside the shell before starting your
editor.
