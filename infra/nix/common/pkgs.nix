let
	# Pull new Rust packages
	moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/9b11a87c0cc54e308fa83aac5b4ee1816d5418a2.tar.gz);

	# Overlay package
	pkgs = import (fetchTarball {
		url = "https://github.com/NixOS/nixpkgs/archive/refs/tags/23.05.tar.gz";
	}) { overlays = [ moz_overlay ]; };
in
	pkgs

