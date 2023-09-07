# nix-build docker.nix --option system-features kvm

# let
# 	pkgs = import ../../nix/common/pkgs.nix;
# 	image = pkgs.dockerTools.buildLayeredImage {
# 		name = "apache-traffic-server";
# 		tag = "latest";

# 		fromImage = "debian:12.1";

# 		contents = [ pkgs.trafficserver ];

# 		extraCommands = ''
# 			#!${pkgs.runtimeShell}
# 			${pkgs.dockerTools.shadowSetup}

# 			groupadd -r trafficserver
# 			useradd -r -g trafficserver trafficserver
# 			mkdir -p /var/log/trafficserver /run/trafficserver
# 			chown -R trafficserver:trafficserver /var/log/trafficserver /run/trafficserver
# 		'';

# 		config = {
# 			Entrypoint = [ "${pkgs.trafficserver}/bin/traffic_server" ];
# 			User = "1000:1000";
# 		};
# 	};
# in image

let
	pkgs = import ../../nix/common/pkgs.nix;
	image = pkgs.dockerTools.buildImage {
		name = "apache-traffic-server";
		tag = "latest";

		fromImageName = "debian";
		fromImageTag = "12.1";

		copyToRoot = pkgs.buildEnv {
			name = "image-root";
			paths = [ pkgs.trafficserver ];
			pathsToLink = [ "/bin" ];
		};

		runAsRoot = ''
			#!${pkgs.runtimeShell}
			${pkgs.dockerTools.shadowSetup}

			groupadd -r trafficserver
			useradd -r -g trafficserver trafficserver
			mkdir -p /var/log/trafficserver /run/trafficserver /etc/trafficserver
			chown -R trafficserver:trafficserver /var/log/trafficserver /run/trafficserver /etc/trafficserver
		'';

		config = {
			Entrypoint = [ "${pkgs.trafficserver}/bin/traffic_server" ];
			User = "trafficserver:trafficserver";
		};
	};
in image

