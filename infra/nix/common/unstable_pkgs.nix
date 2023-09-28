let
	# Pull new Rust packages
	moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/e6ca26fe8b9df914d4567604e426fbc185d9ef3e.tar.gz);

	# Bleeding edge packages for those that need to be up to date with the
	# latest. We pin this to a specific commit instead of using `master` so
	# we're not building environments against a moving target & improves
	# reproducability.
	pkgs = import (fetchTarball {
		url = "https://github.com/NixOS/nixpkgs/archive/462f4a45aa5f988ae94156fdc9a61b7d3d0f7fbf.tar.gz";
	}) { overlays = [ moz_overlay ]; };
in
	pkgs

