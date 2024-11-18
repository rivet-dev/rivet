#!/usr/bin/env -S deno run --allow-net --allow-env --allow-read --allow-write

import { resolve } from "@std/path";
import { exists } from "@std/fs";
import dedent from "dedent";

interface Service {
	name: string;
	command: string;

	/** Don't redirect logs. This will cause logs to log directly to stdout. */
	noRedirectLogs?: boolean;

	/** If a dir should be created in `/data/{service}`. */
	dataDir?: boolean;

	/**
	 * If this needs to run as root user. This will prevent creating a
	 * dedicated user for the service.
	 */
	rootUser?: boolean;

	/**
	 * Command to run to check this service's health. Will prevent services
	 * that depend on this service from starting until this command has
	 * succeeded.
	 */
	healthCheckCommand?: string;

	/**
	 * Wait for these dependencies to start before starting this service.
	 *
	 * See `healthCheckCommand`.
	 */
	dependencies?: string[];

	/**
	 * Ports exposed by this service.
	 *
	 * Purely for documentation purposes.
	 */
	ports: Record<string, number>;
}

const services: Service[] = [
	// Dependencies take a long time to start, so let the user know that
	// something is happening
	{
		name: "rivet-start-message",
		// Sleep for infinity since this service will be restarted if it exits
		command:
			"echo 'Starting...'; sleep infinity",
		noRedirectLogs: true,
		rootUser: true,
		ports: {},
	},

	{
		name: "rivet-server",
		command: "rivet-server start",
		noRedirectLogs: true,
		healthCheckCommand: "curl -f http://127.0.0.1:8090/health/liveness",
		dependencies: [
			"cockroachdb",
			"redis",
			"clickhouse",
			"nats",
			"seaweedfs",
		],
		ports: {
			api: 8080,
			apiInternal: 8081,
			pegboard: 8082,
			health: 8090,
			metrics: 8091,
		},
	},

	{
		name: "rivet-client",
		command: "rivet-client -c /etc/rivet-client/config.yaml",
		dependencies: [
			"rivet-server",
		],
		dataDir: true,
		rootUser: true,
		ports: {
			runner: 7080,
			metrics: 7090,
		},
	},

	{
		name: "cockroachdb",
		command:
			"cockroach start-single-node --insecure --store=/data/cockroachdb --http-port 9200",
		dataDir: true,
		healthCheckCommand: "curl -f http://127.0.0.1:9200/health?ready=1",
		ports: { http: 9200, sql: 26257 },
	},

	{
		name: "redis",
		command:
			"redis-server --dir /data/redis --requirepass password --save 60 1 --appendonly yes",
		dataDir: true,
		healthCheckCommand: "redis-cli ping",
		ports: { default: 6379 },
	},

	{
		name: "clickhouse",
		command:
			"clickhouse-server --config-file=/etc/clickhouse-server/config.xml",
		dataDir: true,
		healthCheckCommand:
			"clickhouse-client --port 9302 --password default --query 'SELECT 1'",
		ports: {
			http: 9300,
			https: 9301,
			tcp: 9302,
			interserver_http: 9303,
			metrics: 9304,
			odbc: 9305,
			mysql: 9306,
			keeper: 9307,
			raft: 9308,
		},
	},

	{
		name: "nats",
		command: "nats-server",
		healthCheckCommand: "nc -z -w 1 127.0.0.1 4222",
		ports: { default: 4222 },
	},

	{
		name: "seaweedfs",
		// raftHashicorp speeds up initial leader election
		command: `
            weed server \
                -dir /data/seaweedfs \
                -master.port=9402 \
				-master.raftHashicorp \
                -volume.port 9400 \
                -filer.port=9403 \
                -s3 \
                -s3.config /etc/seaweedfs/s3.json \
                -s3.port=9000 \
                -s3.allowEmptyFolder=false \
                -s3.allowDeleteBucketNotEmpty=false
		`,
		dataDir: true,
		healthCheckCommand: "curl -f http://127.0.0.1:9000/healthz",
		ports: {
			volume: 9400,
			master: 9401,
			filer: 9403,

			// 9000 is the standard port for S3 in development (based on Minio)
			s3: 9000,
		},
	},

	{
		name: "vector-server",
		command: "vector -C /etc/vector-server",
		dataDir: true,
		ports: {
			api: 9500,
			source_vector: 6000,
			source_tcp_json: 6100,
			sink_prometheus_metrics: 9598,
		},
	},

	{
		name: "vector-client",
		command: "vector -C /etc/vector-client",
		dataDir: true,
		ports: {
			api: 9510,
		},
	},
];

const basePath = "/etc/s6-overlay";
const rcPath = resolve(basePath, "s6-rc.d");
const scriptsPath = resolve(basePath, "scripts");

/**
 * Create the `fs-setup` service.
 *
 * This sets up the filesystem to run the services:
 *
 * - Create `/data/{service}` dir
 * - Create `/var/log/{service}` dir
 */
async function createFsSetupService() {
	// MARK: Service
	const servicePath = `${rcPath}/fs-setup`;
	await Deno.mkdir(servicePath, { recursive: true });

	await Deno.writeTextFile(`${servicePath}/type`, "oneshot");

	await Deno.writeTextFile(
		`${servicePath}/up`,
		`${scriptsPath}/fs-setup.sh`,
	);

	// MARK: Script
	// Generate required files for each service
	let scriptContent = `#!/bin/sh\n`;
	for (const service of services) {
		// Data
		if (service.dataDir) {
			scriptContent += `mkdir -p /data/${service.name}\n`;
			if (!service.rootUser) {
				scriptContent +=
					`chown ${service.name}:${service.name} /data/${service.name}\n`;
			}
		}

		// Logs
		if (!service.noRedirectLogs) {
			scriptContent += `mkdir -p /var/log/${service.name}\n`;
			scriptContent += `chown nobody:nogroup /var/log/${service.name}\n`;
			scriptContent += `chmod 02755 /var/log/${service.name}\n`;
		}
	}
	await Deno.writeTextFile(
		`${scriptsPath}/fs-setup.sh`,
		scriptContent,
	);
	await Deno.chmod(`${scriptsPath}/fs-setup.sh`, 0o755);
}

/**
 * Create the `hosts-setup` service.
 *
 * This sets up the hosts file with DNS entries for each service.
 *
 * We do this in order to make configs easier to read to understand which
 * service is being connected to.
 *
 * This also makes the configuration files consistent with networking inside a
 * Docker Compose in order to keep configurations consistent.
 */
async function createHostsSetupService() {
	// MARK: Service
	const servicePath = `${rcPath}/hosts-setup`;
	await Deno.mkdir(servicePath, { recursive: true });

	await Deno.writeTextFile(`${servicePath}/type`, "oneshot");

	await Deno.writeTextFile(
		`${servicePath}/up`,
		`${scriptsPath}/hosts-setup.sh`,
	);

	// MARK: Script
	// Generate hosts file entries for each service
	let scriptContent = `#!/bin/sh\n`;
	for (const service of services) {
		scriptContent += `echo "127.0.0.1 ${service.name}" >> /etc/hosts\n`;
	}
	await Deno.writeTextFile(
		`${scriptsPath}/hosts-setup.sh`,
		scriptContent,
	);
	await Deno.chmod(`${scriptsPath}/hosts-setup.sh`, 0o755);
}

/**
 * Create a service & log service for each service.
 *
 * The log service writes the output to the file system for each system
 * diagnosis.
 */
async function createServiceFiles(service: Service) {
	// MARK: Service
	const servicePath = `${rcPath}/${service.name}`;
	await Deno.mkdir(servicePath, { recursive: true });

	await Deno.writeTextFile(`${servicePath}/type`, "longrun");

	if (!service.noRedirectLogs) {
		await Deno.writeTextFile(
			`${servicePath}/producer-for`,
			`${service.name}-log`,
		);
	}

	await Deno.mkdir(`${servicePath}/dependencies.d`, { recursive: true });
	await Deno.writeTextFile(`${servicePath}/dependencies.d/base`, "");
	await Deno.writeTextFile(
		`${servicePath}/dependencies.d/fs-setup`,
		"",
	);
	await Deno.writeTextFile(
		`${servicePath}/dependencies.d/hosts-setup`,
		"",
	);
	if (service.dependencies) {
		for (const dependency of service.dependencies) {
			await Deno.writeTextFile(
				`${servicePath}/dependencies.d/${dependency}`,
				"",
			);
			await Deno.writeTextFile(
				`${servicePath}/dependencies.d/${dependency}-health`,
				"",
			);
		}
	}

	if (service.rootUser) {
		await Deno.writeTextFile(
			`${servicePath}/run`,
			dedent`#!/bin/sh
			exec 2>&1
			exec "${scriptsPath}/${service.name}-run.sh"
			`,
		);
	} else {
		await Deno.writeTextFile(
			`${servicePath}/run`,
			dedent`#!/bin/sh
			exec 2>&1
			exec su ${service.name} -c "${scriptsPath}/${service.name}-run.sh"
			`,
		);
	}
	await Deno.chmod(`${servicePath}/run`, 0o755);

	// Write script
	const runScriptPath = `${scriptsPath}/${service.name}-run.sh`;
	await Deno.writeTextFile(
		runScriptPath,
		`#!/bin/sh\n${service.command}`,
	);
	await Deno.chmod(runScriptPath, 0o755);

	// MARK: Logger
	if (!service.noRedirectLogs) {
		const logPath = `${rcPath}/${service.name}-log`;
		await Deno.mkdir(logPath, { recursive: true });

		await Deno.writeTextFile(`${logPath}/type`, "longrun");

		await Deno.writeTextFile(`${logPath}/consumer-for`, `${service.name}`);

		await Deno.writeTextFile(
			`${logPath}/pipeline-name`,
			`${service.name}-pipeline`,
		);

		await Deno.mkdir(`${logPath}/dependencies.d`, { recursive: true });
		await Deno.writeTextFile(
			`${logPath}/dependencies.d/fs-setup`,
			"",
		);

		await Deno.writeTextFile(
			`${logPath}/run`,
			dedent`
			#!/bin/sh
			exec logutil-service /var/log/${service.name}
			`,
		);
		await Deno.chmod(`${logPath}/run`, 0o755);
	}

	// MARK: Health Check
	if (service.healthCheckCommand) {
		// Write service
		const healthcheckPath = `${rcPath}/${service.name}-health`;
		await Deno.mkdir(healthcheckPath, { recursive: true });

		await Deno.writeTextFile(`${healthcheckPath}/type`, "oneshot");

		await Deno.writeTextFile(
			`${healthcheckPath}/up`,
			dedent`
			${scriptsPath}/${service.name}-healthcheck-loop.sh
            `,
		);
		await Deno.chmod(`${healthcheckPath}/up`, 0o755);

		// Write script
		const healthcheckScriptPath =
			`${scriptsPath}/${service.name}-healthcheck.sh`;
		await Deno.writeTextFile(
			healthcheckScriptPath,
			dedent`
            #!/bin/sh
            ${service.healthCheckCommand}
            `,
		);
		await Deno.chmod(healthcheckScriptPath, 0o755);

		// Write checker loop
		const healthcheckLoopScriptPath =
			`${scriptsPath}/${service.name}-healthcheck-loop.sh`;
		await Deno.writeTextFile(
			healthcheckLoopScriptPath,
			dedent`
            #!/bin/sh
			exec > /var/log/${service.name}-health.log 2>&1

			start_time=\$(date +%s%3N)
            while ! (echo 'Running health check'; ${scriptsPath}/${service.name}-healthcheck.sh); do
				echo 'Health check failed'
                sleep 0.25
            done

			end_time=\$(date +%s%3N)
			elapsed_time=\$((end_time - start_time))
			echo "Health check passed in \${elapsed_time}ms."

            exit 0
            `,
		);
		await Deno.chmod(healthcheckLoopScriptPath, 0o755);
	}

	// MARK: Register service to be started
	if (service.noRedirectLogs) {
		await Deno.writeTextFile(
			`${rcPath}/user/contents.d/${service.name}`,
			"",
		);
	} else {
		await Deno.writeTextFile(
			`${rcPath}/user/contents.d/${service.name}-pipeline`,
			"",
		);
	}
}

async function generateConfigurations() {
	await Deno.mkdir(`${rcPath}/user/contents.d`, { recursive: true });
	await Deno.mkdir(`${basePath}/scripts`, { recursive: true });

	await createFsSetupService();
	await createHostsSetupService();

	for (const service of services) {
		await createServiceFiles(service);
	}
}

generateConfigurations();
