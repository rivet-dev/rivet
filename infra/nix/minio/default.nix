let
	pkgs = import ../common/pkgs.nix;
in
with pkgs;
mkShell {
	buildInputs = [ minio ];

	phases = [ "installPhase" ];

	installPhase = ''
	mkdir -p $out/bin
	ln -s ${minio}/bin/minio $out/bin/minio
	'';
}

