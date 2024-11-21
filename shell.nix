# Cross-platform Rust Setup: https://zeroes.dev/p/nix-recipe-for-rust/

let
	# Pull new Rust packages
	moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/9b11a87c0cc54e308fa83aac5b4ee1816d5418a2.tar.gz);

	# Overlay package
	pkgs = import (fetchTarball {
		url = "https://github.com/NixOS/nixpkgs/archive/refs/tags/23.05.tar.gz";
	}) { overlays = [ moz_overlay ]; };
	
	foundationDbShellHook = ''
		# Set LIBCLANG_PATH to point directly to the library
		export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
	'';
in
	pkgs.mkShell {
		name = "rivet";

		buildInputs = with pkgs; [
			# Tools
			cloc
			curl
			docker-client  # Standardize client CLI since older clients have breaking changes
			git-lfs
			jq
			openssh  # ssh-keygen

			python310Packages.detect-secrets

			# Compilers
			clang
			llvm
			lld
			pkg-config
			pkgs.latest.rustChannels.stable.rust

			# Libraries
			openssl

			# Autocomplete
			bashInteractive
			bash-completion

			# Utilities
			netcat

			# FoundationDB
			llvmPackages.libclang
			fdbPackages.foundationdb71

			# Fixes "cannot change locale" warning
			glibcLocales
		]
			++ (
				pkgs.lib.optionals stdenv.isDarwin [
					libiconv  # See https://stackoverflow.com/a/69732679
					darwin.apple_sdk.frameworks.Security
					darwin.apple_sdk.frameworks.CoreServices
					darwin.apple_sdk.frameworks.CoreFoundation
					darwin.apple_sdk.frameworks.Foundation
				]
			);
		shellHook = ''
			# Setup Git LFS
			git lfs install --manual > /dev/null
			
			# Install autocomplete
			source ${pkgs.bash-completion}/share/bash-completion/bash_completion

			# Fix dynamic library path to fix issue with Python and FoundationDB
			export LD_LIBRARY_PATH="${pkgs.clang}/resource-root/lib:${pkgs.lib.strings.makeLibraryPath [ pkgs.openssl ]}:${pkgs.lib.strings.makeLibraryPath [ pkgs.fdbPackages.foundationdb71 ]}"

			# Set default Rust flags to match the Rust flags used inside of Bolt.
			#
			# If these don't match, then the build cache is purged any time Rust is ran from Bolt.
			export RUSTFLAGS="--cfg tokio_unstable"

			${foundationDbShellHook}
		'';
	}
