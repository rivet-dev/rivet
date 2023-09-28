let
	pkgs = import ../common/pkgs.nix;
in
pkgs.rustPlatform.buildRustPackage rec {
	name = "bolt";
	pname = "bolt";
	cargoLock.lockFile = ../../../lib/bolt/Cargo.lock;
	# src = pkgs.lib.cleanSource ../../../lib/bolt;
	src = ../../../lib/bolt;

	# See https://artemis.sh/2023/07/08/nix-rust-project-with-git-dependencies.html
	cargoLock.outputHashes = {
		"async-posthog-0.2.3" = "sha256-v61uSvp528KBzO6dJUcicp42AkxoU9rydQtpN0WzrLM=";
		"rivet-term-0.1.0" = "sha256-AKJ1WeKIkJ93Do22pQSucK+5Gvj662MP2lIdEIIybVw=";
	};
}

