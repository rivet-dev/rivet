let
	pkgs = import ../common/pkgs.nix;
in
with pkgs;
mkShell {
	buildInputs = [ trafficserver ];

	phases = [ "installPhase" ];

	installPhase = let
		bins = [
			"traffic_cache_tool"
			"traffic_crashlog"
			"traffic_ctl"
			"traffic_layout"
			"traffic_logcat"
			"traffic_logstats"
			"traffic_manager"
			"traffic_server"
			"traffic_top"
			"traffic_via"
			"trafficserver"
			"tspush"
			"tsxs"
		];
		lnCmds = lib.concatStringsSep "\n" (map (x: "ln -s ${trafficserver}/bin/${x} $out/bin/${x}") bins);
	in
	''
	mkdir -p $out/bin
	${lnCmds}
	'';
}

