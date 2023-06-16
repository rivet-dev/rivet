let
	pkgs = import ../common/pkgs.nix;
	clickhouse = import ../pkgs/clickhouse.nix { inherit (pkgs) stdenv fetchurl lib; };
in
pkgs.mkShell {
	buildInputs = [ clickhouse ];

	phases = [ "installPhase" ];

	installPhase = ''
	mkdir -p $out
	ln -s ${clickhouse}/bin $out/bin
	'';
}

