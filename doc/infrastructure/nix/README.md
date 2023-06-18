# Nix

## What is Nix?

Nix (not to be confused with Unix and Linux) is a reliable build system. Nix provides a really powerful deterministic build tool & package management system. It enables us to build tools and expose them to environments without having to install them on the whole system. It helps ensure that all developers are running the same versions of the software we use across all platforms.

## What do we use Nix for?

**Development environment**

All commands relating to Rivet need to be ran in a Nix shell. When you run `nix-shell`, the `shell.nix` file will create a new shell with all of the required packages in `$PATH` and add any extra environment variables required to interact with Rivet. This way, everyone is running the exact same version of the software with the exact same environment without having to run a virtual machine to develop with Rivet.

**Services**

Nix is used to build dependencies in production instead of using apt or another system package manager to install them.

Because Nix supports aggressive caching, we can use the same Nix configs to download cached prebuilt binaries.

