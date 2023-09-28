# Cross-platofrm Rust Setup: https://zeroes.dev/p/nix-recipe-for-rust/

let
	# Include most recent Rust builds
	moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/e6ca26fe8b9df914d4567604e426fbc185d9ef3e.tar.gz);

	# If you need a newer version of packages, use unstablePkgs.
	pkgs = import (fetchTarball {
		url = "https://github.com/NixOS/nixpkgs/archive/refs/tags/23.05.tar.gz";
	}) { overlays = [ moz_overlay ]; };

	# Bleeding edge packages for those that need to be up to date with the
	# latest. We pin this to a specific commit instead of using `master` so
	# we're not building environments against a moving target & improves
	# reproducability.
	unstablePkgs = import (fetchTarball {
		url = "https://github.com/NixOS/nixpkgs/archive/462f4a45aa5f988ae94156fdc9a61b7d3d0f7fbf.tar.gz";
	}) { overlays = [ moz_overlay ]; };

	custom_clickhouse = import ./infra/nix/pkgs/clickhouse.nix { inherit (pkgs) stdenv fetchurl lib; };
in
	pkgs.mkShell {
		name = "rivet";

		buildInputs = with pkgs; [
			# Shell
			bash

			# Kubernetes tools
			k3d
			kubectl
			kubernetes-helm

			# Clouds
			awscli2

			# Infrastructure
			consul
			nomad
			terraform

			# Tools
			cloc
			curl
			docker-client
			git  # Bolt relies functionality only available in newer versions of Bolt
			rsync
			traefik  # Used to proxy requests in Bolt
			cloudflared
			go-migrate
			jq
			
			# Databases
			postgresql  # For psql
			custom_clickhouse  # For ClickHouse CLI
			redis  # For the redis-cli

			# Runtimes
			nodejs  # Required for Fern

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

			# Fixes "cannot change locale" warning
			glibcLocales
		] ++ (
			pkgs.lib.optionals stdenv.isDarwin [
				libiconv  # See https://stackoverflow.com/a/69732679
				darwin.apple_sdk.frameworks.Security
				darwin.apple_sdk.frameworks.CoreServices
				darwin.apple_sdk.frameworks.CoreFoundation
				darwin.apple_sdk.frameworks.Foundation
			]
		);
		shellHook = ''
			# Add binaries to path. Prefer debug builds over release builds
			# since release builds are usually the default but debug builds are
			# used for testing things locally.
			export PATH="$PATH:${toString ./target/debug/.}:${toString ./target/release/.}"

			# See https://docs.rs/prost-build/0.8.0/prost_build/#sourcing-protoc
			export PROTOC="${pkgs.protobuf}/bin/protoc"
			export PROTOC_INCLUDE="${pkgs.protobuf}/include"

			
			# Install autocomplete
			source ${pkgs.bash-completion}/share/bash-completion/bash_completion
      		# nomad -autocomplete-install
      		complete -C ${pkgs.nomad}/bin/nomad nomad
      		# consul -autocomplete-install
      		complete -C ${pkgs.consul}/bin/consul consul
      		# terraform -install-autocomplete
      		complete -C ${pkgs.terraform}/bin/terraform terraform
			# awscli
			complete -C aws_completer aws
			# kubectl completion bash
			source <(kubectl completion bash)


			# Fix dynamic library path to fix issue with Python
			export LD_LIBRARY_PATH="${pkgs.clang}/resource-root/lib:${pkgs.lib.strings.makeLibraryPath [ pkgs.openssl ]}"

			# Set default Rust flags to match the Rust flags used inside of Bolt.
			#
			# If these don't match, then the build cache is purged any time Rust is ran from Bolt.
			export RUSTFLAGS="--cfg tokio_unstable"
		'';
	}
