# Nix

## What is Nix?

Nix (not to be confused with Unix and Linux) is a reliable build system. Nix provides a really powerful deterministic package & environment system for installing packages in a specific directory without having to install it on the entire system. It helps ensure that all developers are running the same versions of the software we use across all platforms.

## What do we use Nix for?

All commands relating to Rivet need to be ran in a Nix shell. When you run `nix-shell`, the `shell.nix` file will create a new shell with all of the required packages in `$PATH` and add any extra environment variables required to interact with Rivet. This way, everyone is running the exact same version of the software with the exact same environment without having to run a virtual machine to develop with Rivet.

