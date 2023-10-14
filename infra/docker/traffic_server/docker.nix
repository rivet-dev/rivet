let
	pkgs = import ../../nix/common/pkgs.nix;
	custom_trafficserver = pkgs.trafficserver.override {
		withLuaJIT = true;
	};
	image = pkgs.dockerTools.buildImage {
		name = "apache-traffic-server";
		tag = "latest";

		copyToRoot = pkgs.buildEnv {
			name = "image-root";
			paths = [ custom_trafficserver ];
			pathsToLink = [ "/bin" ];
		};

		runAsRoot = ''
			#!${pkgs.runtimeShell}
			${pkgs.dockerTools.shadowSetup}
			mkdir -p /var/log/trafficserver /run/trafficserver /etc/trafficserver
			chmod 777 /var/log/trafficserver /run/trafficserver /etc/trafficserver
		'';

		config = {
			Entrypoint = [ "${custom_trafficserver}/bin/traffic_server" ];
		};
	};
in image

