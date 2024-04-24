let
	pkgs = import ../common/pkgs.nix;

	# Upgrade Rust
	rustPlatform = pkgs.makeRustPlatform {
		rustc = pkgs.latest.rustChannels.stable.rust;
		cargo = pkgs.latest.rustChannels.stable.rust;
	};

	# Filter file related to Bolt
	projectRoot = ../../..;
	includeDirs = [
		"lib/bolt"
		"lib/s3-util"
		"proto"
		"sdks/full/rust"
	];

	includeFilter = path: type: let
		relPath = pkgs.lib.removePrefix (toString projectRoot + "/") (toString path);
	in
		pkgs.lib.any
			(dir: (pkgs.lib.hasPrefix (dir + "/") relPath) || (pkgs.lib.hasPrefix relPath dir))
			includeDirs;

	filteredSrc = pkgs.lib.cleanSourceWith {
		src = projectRoot;
		filter = includeFilter;
	};
in
rustPlatform.buildRustPackage rec {
	name = "bolt";
	pname = "bolt";

	src = filteredSrc;
	sourceRoot = "source/lib/bolt";
	cargoLock.lockFile = ../../../lib/bolt/Cargo.lock;

	buildInputs = with pkgs; [
		pkg-config
		openssl
	] ++ (
		pkgs.lib.optionals stdenv.isDarwin [
			darwin.apple_sdk.frameworks.SystemConfiguration
		]
	);
	nativeBuildInputs = buildInputs;
	doCheck = false;


	shellHook = ''
		export LD_LIBRARY_PATH="${pkgs.lib.strings.makeLibraryPath [ pkgs.openssl ]}"
		export RUSTFLAGS="--cfg tokio_unstable"
	'';

	# See https://artemis.sh/2023/07/08/nix-rust-project-with-git-dependencies.html
	cargoLock.outputHashes = {
		"async-posthog-0.2.3" = "sha256-v61uSvp528KBzO6dJUcicp42AkxoU9rydQtpN0WzrLM=";
		"rivet-term-0.1.0" = "sha256-OvOu4xYW65YEuAH+BXyIwtWyELmZPvp6n0tSmInEyBY=";
	};
}

