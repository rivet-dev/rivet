let
	pkgs = import ../common/pkgs.nix;
in
with pkgs;
mkShell {
	buildInputs = [ cloudflared ];

	phases = [ "installPhase" ];

	installPhase = ''
	mkdir -p $out/bin
	ln -s ${cloudflared}/bin/cloudflared $out/bin/cloudflared
	'';
}

