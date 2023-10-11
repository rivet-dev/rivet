let
	pkgs = import ../../nix/common/pkgs.nix;
	image = pkgs.dockerTools.buildImage {
		name = "apache-traffic-server";
		tag = "latest";

		copyToRoot = pkgs.buildEnv {
			name = "image-root";
			paths = [ pkgs.trafficserver ];
			pathsToLink = [ "/bin" ];
		};

		runAsRoot = ''
			#!${pkgs.runtimeShell}
			${pkgs.dockerTools.shadowSetup}
			mkdir -p /var/log/trafficserver /run/trafficserver /etc/trafficserver
			chmod 777 /var/log/trafficserver /run/trafficserver /etc/trafficserver
		'';

		config = {
			Entrypoint = [ "${pkgs.trafficserver}/bin/traffic_server" ];
		};
	};
in image

