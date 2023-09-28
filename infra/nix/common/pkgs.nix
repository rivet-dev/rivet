let
	# Pull new Rust packages
	moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/e6ca26fe8b9df914d4567604e426fbc185d9ef3e.tar.gz);

	# Overlay package
	pkgs = import (fetchTarball {
		url = "https://github.com/NixOS/nixpkgs/archive/refs/tags/23.05.tar.gz";
	}) { overlays = [ moz_overlay ]; };
in
	pkgs

