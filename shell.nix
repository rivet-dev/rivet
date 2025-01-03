# Cross-platform Rust Setup: https://zeroes.dev/p/nix-recipe-for-rust/

let

	# Pull new Rust packages
	moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/ad7af231a95acf65ccc4afa0c766f5c0674ad3f1.tar.gz);

	# Overlay package
	pkgs = import (fetchTarball {
		# Required for https://github.com/NixOS/nixpkgs/issues/208951#issuecomment-2143431252
		url = "https://github.com/NixOS/nixpkgs/archive/refs/tags/24.11-beta.tar.gz";
	}) { overlays = [ moz_overlay ]; };

	# TODO(RVT-4163): FoundationDB Nix is only supported on Linux
	isFdbSupported = pkgs.stdenv.isLinux && pkgs.stdenv.hostPlatform.system == "x86_64-linux";
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
			protobuf

			# Autocomplete
			bashInteractive
			bash-completion

			# Utilities
			netcat

			# FoundationDB
			llvmPackages.libclang

			# Fixes "cannot change locale" warning
			glibcLocales

			just
		] ++ (
			# Use the global variable to check if FoundationDB is supported
			pkgs.lib.optionals isFdbSupported [
				fdbPackages.foundationdb71
			]
		) ++ (
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

			# See https://docs.rs/prost-build/0.8.0/prost_build/#sourcing-protoc
			export PROTOC="${pkgs.protobuf}/bin/protoc"
			export PROTOC_INCLUDE="${pkgs.protobuf}/include"
			
			# Install autocomplete
			source ${pkgs.bash-completion}/share/bash-completion/bash_completion
			source ${pkgs.just}/share/bash-completion/completions/just.bash

			export LD_LIBRARY_PATH="${pkgs.clang}/resource-root/lib"
			export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.strings.makeLibraryPath [ pkgs.openssl ]}"

			# tokio_unstable required to build Rivet, so force all cargo
			# commands to use this flag.
			export RUSTFLAGS="--cfg tokio_unstable"

			export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"

			${if isFdbSupported then ''
				export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.strings.makeLibraryPath [ pkgs.fdbPackages.foundationdb71 ]}"
			'' else ''
				# Manually check the FDB version
				if [ -z "$(command -v fdbcli)" ]; then
					echo "WARNING: FoundationDB CLI is not installed. Please install FoundationDB 7.1."
				elif ! fdbcli --version 2>/dev/null | grep -q "FoundationDB CLI 7.1"; then
					echo "WARNING: FoundationDB CLI version is incorrect. Please install FoundationDB 7.1."
				fi
			''}
		'';
	}
