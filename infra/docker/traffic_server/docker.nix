let
	pkgs = import ../../nix/common/pkgs.nix;
	image = pkgs.dockerTools.buildLayeredImage {
		name = "apache-traffic-server";
		tag = "latest";
		contents = [ pkgs.trafficserver ];
		config = {
			Cmd = [ "${pkgs.trafficserver}/bin/traffic_server" ];
		};
	};
in image

