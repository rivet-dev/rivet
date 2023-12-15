# Cross-platform Rust Setup: https://zeroes.dev/p/nix-recipe-for-rust/

let
	pkgs = import ./infra/nix/common/pkgs.nix;
	unstablePkgs = import ./infra/nix/common/unstable_pkgs.nix;

	custom_clickhouse = import ./infra/nix/pkgs/clickhouse.nix { inherit (pkgs) stdenv fetchurl lib; };
	custom_bolt = import ./infra/nix/bolt/default.nix;

	useSccache = builtins.getEnv "USE_SCCACHE" == "1";
	extraInputs = if useSccache then [ unstablePkgs.sccache ] else [];
	sccacheShellHook = if useSccache then ''
		export RUSTC_WRAPPER=sccache
		export SCCACHE_BUCKET=${builtins.getEnv "SCCACHE_BUCKET"}
		export SCCACHE_ENDPOINT=${builtins.getEnv "SCCACHE_ENDPOINT"}
		export SCCACHE_REGION=${builtins.getEnv "SCCACHE_REGION"}
		export AWS_ACCESS_KEY_ID=${builtins.getEnv "AWS_ACCESS_KEY_ID"}
		export AWS_SECRET_ACCESS_KEY=${builtins.getEnv "AWS_SECRET_ACCESS_KEY"}
	'' else "";
in
	pkgs.mkShell {
		name = "rivet";

		buildInputs = with pkgs; [
			# Kubernetes tools
			k3d
			kubectl
			kubernetes-helm

			# Clouds
			awscli2

			# Infrastructure
			terraform

			# Tools
			custom_bolt
			cloc
			curl
			docker-client  # Standardize client CLI since older clients have breaking changes
			git  # Bolt relies functionality only available in newer versions of Bolt
			git-lfs
			go-migrate
			jq
			openssh  # ssh-keygen

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
		] ++ extraInputs ++ (
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
			git lfs install > /dev/null

			# Add binaries to path so we can use a locally built copy of Bolt.
			export PATH="${toString ./target/debug/.}:${toString ./target/release/.}:$PATH"

			# See https://docs.rs/prost-build/0.8.0/prost_build/#sourcing-protoc
			export PROTOC="${pkgs.protobuf}/bin/protoc"
			export PROTOC_INCLUDE="${pkgs.protobuf}/include"
			
			# Install autocomplete
			source ${pkgs.bash-completion}/share/bash-completion/bash_completion
      		# terraform -install-autocomplete
      		complete -C ${pkgs.terraform}/bin/terraform terraform
			# awscli
			complete -C aws_completer aws
			# kubectl completion bash
			source <(kubectl completion bash)

			# Automatically connect to correct cluster
			alias kubectl='KUBECONFIG=$(bolt output project-root)/gen/k8s/kubeconfig/$(bolt output namespace).yml kubectl'
			alias helm='KUBECONFIG=$(bolt output project-root)/gen/k8s/kubeconfig/$(bolt output namespace).yml helm'

			# Fix dynamic library path to fix issue with Python
			export LD_LIBRARY_PATH="${pkgs.clang}/resource-root/lib:${pkgs.lib.strings.makeLibraryPath [ pkgs.openssl ]}"

			# Set default Rust flags to match the Rust flags used inside of Bolt.
			#
			# If these don't match, then the build cache is purged any time Rust is ran from Bolt.
			export RUSTFLAGS="--cfg tokio_unstable"

			${sccacheShellHook}
		'';
	}


