import * as yaml from "js-yaml";
import { CORE_NETWORK_NAME, type TemplateContext } from "./context";

export function generateDockerCompose(context: TemplateContext) {
	const config = context.config;

	// Services configuration
	const services: Record<string, any> = {};
	const networks: Record<string, any> = {};
	const volumes: Record<string, any> = {};

	const allDcEnginePeerNetworkNames = config.datacenters.map(
		(datacenter) =>
			`${context.getDatacenterNetworkName(datacenter.name)}-engine-peer`,
	);
	const allDcToCoreNetworkNames = config.datacenters.map(
		(datacenter) =>
			`${context.getDatacenterNetworkName(datacenter.name)}-to-core`,
	);

	// Core services
	networks[CORE_NETWORK_NAME] = {
		driver: "bridge",
	};

	// ClickHouse
	services["clickhouse"] = {
		restart: "unless-stopped",
		image: "clickhouse/clickhouse-server:25.1.5",
		volumes: [
			"clickhouse-data:/var/lib/clickhouse",
			`./${context.getCoreServicePath("clickhouse")}/config.xml:/etc/clickhouse-server/config.d/config.xml`,
			`./${context.getCoreServicePath("clickhouse")}/users.xml:/etc/clickhouse-server/users.d/users.xml`,
			`./${context.getCoreServicePath("clickhouse")}/client-config.xml:/etc/clickhouse-client/config.xml`,
			`./${context.getCoreServicePath("clickhouse")}/init:/docker-entrypoint-initdb.d`,
		],
		environment: [
			//Run migrations on startup
			"CLICKHOUSE_ALWAYS_RUN_INITDB_SCRIPTS=true",
			"CLICKHOUSE_USER=default",
			"CLICKHOUSE_PASSWORD=default",
			"CLICKHOUSE_HTTP_PORT=9300",
			"CLICKHOUSE_TCP_PORT=9301",
		],
		networks: [...allDcToCoreNetworkNames, CORE_NETWORK_NAME],
		healthcheck: {
			test: [
				"CMD",
				"clickhouse-client",
				"--host",
				"127.0.0.1",
				"--port",
				"9301",
				"--user",
				"system",
				"--password",
				"default",
				"--query",
				"SELECT 1",
			],
			interval: "2s",
			timeout: "10s",
			retries: 10,
		},
	};
	volumes["clickhouse-data"] = null;

	// Prometheus
	services["prometheus"] = {
		restart: "unless-stopped",
		image: "prom/prometheus:latest",
		volumes: [
			"prometheus-data:/prometheus",
			`./${context.getCoreServicePath("prometheus")}/prometheus.yml:/etc/prometheus/prometheus.yml`,
		],
		command: [
			"--config.file=/etc/prometheus/prometheus.yml",
			"--storage.tsdb.path=/prometheus",
			"--web.console.libraries=/usr/share/prometheus/console_libraries",
			"--web.console.templates=/usr/share/prometheus/consoles",
			"--web.enable-remote-write-receiver",
		],
		ports: ["9090:9090"],
		networks: [...allDcToCoreNetworkNames, CORE_NETWORK_NAME],
		healthcheck: {
			test: ["CMD", "wget", "--spider", "-q", "http://127.0.0.1:9090/-/healthy"],
			interval: "5s",
			timeout: "3s",
			retries: 3,
		},
	};
	volumes["prometheus-data"] = null;

	// Grafana
	services["grafana"] = {
		image: "grafana/grafana:11.5.2",
		volumes: [
			"grafana-data:/var/lib/grafana",
			`./${context.getCoreServicePath("grafana")}/grafana.ini:/etc/grafana/grafana.ini`,
			`./${context.getCoreServicePath("grafana")}/provisioning:/etc/grafana/provisioning`,
			`./${context.getCoreServicePath("grafana")}/dashboards:/var/lib/grafana/dashboards`,
		],
		environment: ["GF_INSTALL_PLUGINS=grafana-clickhouse-datasource"],
		ports: ["3100:3000"],
		networks: [CORE_NETWORK_NAME],
		depends_on: {
			clickhouse: { condition: "service_healthy" },
			prometheus: { condition: "service_healthy" },
		},
	};
	volumes["grafana-data"] = null;

	// Generate services for each datacenter
	config.datacenters.forEach((datacenter, idx) => {
		const isPrimary = idx === 0;

		const dcNetworkName = context.getDatacenterNetworkName(datacenter.name);
		const dcEnginePeerNetworkName = `${dcNetworkName}-engine-peer`;
		const dcToCoreNetworkName = `${dcNetworkName}-to-core`;

		//const natsServiceName = context.getServiceName("nats", datacenter.name);
		const vectorServerServiceName = context.getServiceName(
			"vector-server",
			datacenter.name,
		);
		const vectorClientServiceName = context.getServiceName(
			"vector-client",
			datacenter.name,
		);
		const otelCollectorServiceName = context.getServiceName(
			"otel-collector",
			datacenter.name,
		);
		const postgresServiceName = context.getServiceName(
			"postgres",
			datacenter.name,
		);
		const shellServiceName = context.getServiceName(
			"rivet-shell",
			datacenter.name,
		);

		networks[dcNetworkName] = {
			driver: "bridge",
		};

		networks[dcEnginePeerNetworkName] = {
			driver: "bridge",
		};

		networks[dcToCoreNetworkName] = {
			driver: "bridge",
		};

		//services[natsServiceName] = {
		//   restart: "unless-stopped",
		//   image: "nats:2.10.22-scratch",
		//   networks: [dcNetworkName],
		//   ports: isPrimary ? [`4222:4222`] : undefined,
		//   healthcheck: {
		//      test: ["CMD", "nats-server", "--health"],
		//      interval: "2s",
		//      timeout: "10s",
		//      retries: 10,
		//   },
		//};

		const postgresVolumeName = context.getVolumeName(
			"postgres",
			datacenter.name,
		);
		services[postgresServiceName] = {
			restart: "unless-stopped",
			image: "postgres:17-alpine",
			environment: [
				"POSTGRES_USER=postgres",
				"POSTGRES_PASSWORD=postgres",
				"POSTGRES_DB=postgres",
			],
			volumes: [
				`./${context.getDatacenterServicePath("postgres", datacenter.name)}/init-db.sh:/docker-entrypoint-initdb.d/init-db.sh`,
				`${postgresVolumeName}:/var/lib/postgresql/data`,
			],
			ports: isPrimary ? [`5432:5432`] : undefined,
			healthcheck: {
				test: ["CMD-SHELL", "pg_isready -U postgres -d postgres"],
				interval: "2s",
				timeout: "10s",
				retries: 10,
			},
			networks: [dcNetworkName],
		};
		volumes[postgresVolumeName] = null;

		services[shellServiceName] = {
			build: {
				context: "../../..",
				dockerfile: "engine/docker/universal/Dockerfile",
				target: "engine-full",
				args: {
					BUILD_FRONTEND: "true",
				},
			},
			platform: "linux/amd64",
			restart: "unless-stopped",
			entrypoint: "sleep",
			command: "infinity",
			stop_grace_period: "0s",
			depends_on: {
				//[natsServiceName]: { condition: "service_healthy" },
				[postgresServiceName]: { condition: "service_healthy" },
			},
			volumes: [
				`./${context.getDatacenterServicePath("rivet-engine", datacenter.name, 0)}/config.jsonc:/etc/rivet/config.jsonc:ro`,
			],
			networks: [
				dcNetworkName,
				dcToCoreNetworkName,
				...allDcEnginePeerNetworkNames,
			],
		};

		services[vectorServerServiceName] = {
			restart: "unless-stopped",
			image: "timberio/vector:0.48.0-distroless-static",
			command: "-C /etc/vector",
			volumes: [
				`vector-server-data-${datacenter.name}:/var/lib/vector`,
				`./${context.getDatacenterServicePath("vector-server", datacenter.name)}:/etc/vector`,
			],
			networks: [dcNetworkName, dcToCoreNetworkName],
			depends_on: {
				clickhouse: { condition: "service_healthy" },
			},
		};
		volumes[`vector-server-data-${datacenter.name}`] = null;

		services[vectorClientServiceName] = {
			restart: "unless-stopped",
			image: "timberio/vector:0.48.0-distroless-static",
			command: "-C /etc/vector",
			volumes: [
				`vector-client-data-${datacenter.name}:/var/lib/vector`,
				`./${context.getDatacenterServicePath("vector-client", datacenter.name)}:/etc/vector`,
			],
			networks: [dcNetworkName],
			depends_on: {
				[vectorServerServiceName]: { condition: "service_started" },
			},
		};
		volumes[`vector-client-data-${datacenter.name}`] = null;

		services[otelCollectorServiceName] = {
			image: "otel/opentelemetry-collector-contrib:latest",
			restart: "unless-stopped",
			command: "--config=/etc/otel/config.yaml",
			volumes: [
				`./${context.getDatacenterServicePath("otel-collector", datacenter.name)}/config.yaml:/etc/otel/config.yaml:ro`,
			],
			environment: ["CLICKHOUSE_PASSWORD=default"],
			depends_on: {
				clickhouse: { condition: "service_healthy" },
				prometheus: { condition: "service_healthy" },
			},
			networks: [dcNetworkName, dcToCoreNetworkName],
			ports: isPrimary ? ["4317:4317"] : undefined,
		};

		for (let i = 0; i < datacenter.engines; i++) {
			const serviceName = context.getServiceName(
				"rivet-engine",
				datacenter.name,
				i,
			);

			services[serviceName] = {
				build: {
					context: "../../..",
					dockerfile: "engine/docker/universal/Dockerfile",
					target: "engine-full",
					args: {
						BUILD_FRONTEND: "true",
					},
				},
				platform: "linux/amd64",
				restart: "unless-stopped",
				environment: [
					"RUST_LOG_ANSI_COLOR=1",
					"RIVET_OTEL_ENABLED=1",
					"RIVET_OTEL_SAMPLER_RATIO=1",
					`RIVET_OTEL_GRPC_ENDPOINT=http://${context.getServiceHost("otel-collector", datacenter.name)}:4317`,
					// "RUST_LOG=debug,hyper=info",
				],
				stop_grace_period: "0s",
				depends_on: {
					//[natsServiceName]: { condition: "service_healthy" },
					[vectorClientServiceName]: {
						condition: "service_started",
					},
					[otelCollectorServiceName]: {
						condition: "service_started",
					},
					[postgresServiceName]: { condition: "service_healthy" },
				},
				volumes: [
					`./${context.getDatacenterServicePath("rivet-engine", datacenter.name, i)}/config.jsonc:/etc/rivet/config.jsonc:ro`,
				],
				networks: [
					dcNetworkName,
					dcToCoreNetworkName,
					...allDcEnginePeerNetworkNames,
				],
				ports: isPrimary && i === 0 ? [`6420:6420`] : undefined,
				healthcheck: {
					test: ["CMD", "curl", "-f", "http://127.0.0.1:6420/health"],
					interval: "2s",
					timeout: "10s",
					retries: 10,
					start_period: "30s",
				},
			};
		}

		for (let i = 0; i < datacenter.runners; i++) {
			const serviceName = context.getServiceName(
				"runner",
				datacenter.name,
				i,
			);
			const engineServiceName = context.getServiceName(
				"rivet-engine",
				datacenter.name,
				0,
			);

			services[serviceName] = {
				build: {
					context: "../../..",
					dockerfile: "engine/sdks/typescript/test-runner/Dockerfile",
				},
				platform: "linux/amd64",
				restart: "unless-stopped",
				environment: [
					`RIVET_ENDPOINT=http://${context.getServiceHost("rivet-engine", datacenter.name, 0)}:6420`,
					`RIVET_RUNNER_TOTAL_SLOTS=1000000`,
					`AUTOSTART_RUNNER=1`,
					`AUTOCONFIGURE_SERVERLESS=0`
				],
				stop_grace_period: "4s",
				ports: isPrimary && i === 0 ? [`5050:5050`] : undefined,
				depends_on: {
					[engineServiceName]: {
						condition: "service_healthy",
					},
				},
				networks: [dcNetworkName],
			};
		}
	});

	const dockerComposeConfig = {
		services,
		networks,
		volumes,
	};

	// If host networking is requested, set network_mode for all services
	if (context.config.networkMode === "host") {
		for (const svc of Object.values(dockerComposeConfig.services)) {
			svc.network_mode = "host";
			// Remove networks field as it's incompatible with host networking
			if (svc.networks) delete svc.networks;
			// Remove ports since published ports are ignored with host networking
			// and produce warnings in Docker Compose output.
			if (svc.ports) delete svc.ports;
		}
	}

	context.writeFile("docker-compose.yml", yaml.dump(dockerComposeConfig));
}
