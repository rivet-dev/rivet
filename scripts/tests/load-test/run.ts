#!/usr/bin/env tsx

import { spawn } from "node:child_process";
import { parseArgs } from "node:util";

const RIVET_ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://localhost:6420";
const RIVET_TOKEN = process.env.RIVET_TOKEN ?? "dev";
const RIVET_NAMESPACE = process.env.RIVET_NAMESPACE ?? "default";
const RUNNER_NAME_SELECTOR =
	process.env.RUNNER_NAME_SELECTOR ?? "test-runner";

interface LoadTestConfig {
	executor: string;
	startVUs: number;
	stages: string;
	gracefulRampDown: string;
	rivetEndpoint: string;
	rivetToken: string;
	rivetNamespace: string;
	runnerNameSelector: string;
	variation: string;
	variationDuration: number;
	out?: string;
	summaryTrendStats: string;
	quiet: boolean;
	skipHealthCheck: boolean;
}

// Parse command line arguments
function parseArguments(): LoadTestConfig {
	const { values } = parseArgs({
		options: {
			help: { type: "boolean", short: "h" },
			executor: {
				type: "string",
				default: "ramping-vus",
			},
			"start-vus": {
				type: "string",
				default: "0",
			},
			stages: {
				type: "string",
				default: "30s:5,1m:10,30s:0",
			},
			"graceful-rampdown": {
				type: "string",
				default: "30s",
			},
			endpoint: {
				type: "string",
				default: RIVET_ENDPOINT,
			},
			token: {
				type: "string",
				default: RIVET_TOKEN,
			},
			namespace: {
				type: "string",
				default: RIVET_NAMESPACE,
			},
			runner: {
				type: "string",
				default: RUNNER_NAME_SELECTOR,
			},
			variation: {
				type: "string",
				default: "sporadic",
			},
			"variation-duration": {
				type: "string",
				default: "120",
			},
			out: {
				type: "string",
			},
			"summary-trend-stats": {
				type: "string",
				default: "avg,min,med,max,p(90),p(95),p(99)",
			},
			quiet: {
				type: "boolean",
				short: "q",
				default: false,
			},
			"skip-health-check": {
				type: "boolean",
				default: false,
			},
		},
		strict: true,
		allowPositionals: false,
	});

	if (values.help) {
		printHelp();
		process.exit(0);
	}

	const config = {
		executor: values.executor as string,
		startVUs: parseInt(values["start-vus"] as string),
		stages: values.stages as string,
		gracefulRampDown: values["graceful-rampdown"] as string,
		rivetEndpoint: values.endpoint as string,
		rivetToken: values.token as string,
		rivetNamespace: values.namespace as string,
		runnerNameSelector: values.runner as string,
		variation: values.variation as string,
		variationDuration: parseInt(values["variation-duration"] as string),
		out: values.out as string | undefined,
		summaryTrendStats: values["summary-trend-stats"] as string,
		quiet: values.quiet as boolean,
		skipHealthCheck: values["skip-health-check"] as boolean,
	};

	// Validate variation duration against test duration
	if (
		config.variation !== "sporadic" &&
		config.variationDuration > 0
	) {
		const testDuration = calculateTestDuration(config.stages);
		const variationDurationMs = config.variationDuration * 1000;

		if (variationDurationMs > testDuration) {
			console.error(`
❌ Error: Variation duration (${config.variationDuration}s) exceeds total test duration (${Math.floor(testDuration / 1000)}s)

The ${config.variation} variation requires actors to remain active for ${config.variationDuration} seconds,
but your test stages only run for ${Math.floor(testDuration / 1000)} seconds.

Solutions:
1. Reduce --variation-duration to ${Math.floor(testDuration / 1000)} or less
2. Extend your test stages to accommodate the variation duration

Example with extended stages:
  tsx run.ts --variation ${config.variation} --variation-duration ${config.variationDuration} --stages "${Math.ceil(config.variationDuration / 60) + 1}m:5"
`);
			process.exit(1);
		}

		// Warn if variation duration is close to test duration (less than 30s buffer)
		const buffer = testDuration - variationDurationMs;
		if (buffer < 30000) {
			console.warn(`
⚠️  Warning: Variation duration (${config.variationDuration}s) is very close to test duration (${Math.floor(testDuration / 1000)}s)

This leaves only ${Math.floor(buffer / 1000)}s for actor creation and cleanup.
Consider extending your test stages for better results.
`);
		}
	}

	return config;
}

// Calculate total test duration from stages string
function calculateTestDuration(stagesStr: string): number {
	const stages = stagesStr.split(",");
	let totalMs = 0;

	for (const stage of stages) {
		const [duration] = stage.split(":");
		totalMs += parseDuration(duration);
	}

	return totalMs;
}

// Parse duration string (e.g., "1m", "30s", "1h30m") to milliseconds
function parseDuration(duration: string): number {
	let totalMs = 0;
	let currentNumber = "";

	for (let i = 0; i < duration.length; i++) {
		const char = duration[i];

		if (char >= "0" && char <= "9") {
			currentNumber += char;
		} else if (char === "h") {
			totalMs += parseInt(currentNumber || "0") * 60 * 60 * 1000;
			currentNumber = "";
		} else if (char === "m") {
			totalMs += parseInt(currentNumber || "0") * 60 * 1000;
			currentNumber = "";
		} else if (char === "s") {
			totalMs += parseInt(currentNumber || "0") * 1000;
			currentNumber = "";
		}
	}

	// If there's a trailing number without unit, assume seconds
	if (currentNumber) {
		totalMs += parseInt(currentNumber) * 1000;
	}

	return totalMs;
}

function printHelp() {
	console.log(`
Rivet Actor Lifecycle Load Test

A comprehensive load testing tool for Rivet Actors using k6. Tests the complete
actor lifecycle including creation, HTTP routes, WebSocket connections, sleep/wake
cycles, and destruction.

USAGE:
  tsx run.ts [OPTIONS]

PREREQUISITES:
  1. Install k6: https://k6.io/docs/get-started/installation/
  2. Start/configure engine: --endpoint <url> or cd engine/docker/dev && docker-compose up -d
  3. Start test-runner: cd engine/sdks/typescript/test-runner && pnpm start

OPTIONS:
  -h, --help                    Show this help message

  Load Test Configuration:
    --executor <type>           k6 executor type (default: ramping-vus)
                                Options: ramping-vus, constant-vus, shared-iterations
    --start-vus <number>        Initial number of virtual users (default: 0)
    --stages <stages>           Test stages in format "duration:target,..."
                                (default: "30s:5,1m:10,30s:0")
                                Example: "1m:10,2m:20,1m:0" means:
                                  - Ramp up to 10 VUs over 1 minute
                                  - Ramp up to 20 VUs over 2 minutes
                                  - Ramp down to 0 VUs over 1 minute
    --graceful-rampdown <time>  Graceful ramp-down period (default: 30s)

  Rivet Configuration:
    --endpoint <url>            Rivet endpoint (default: http://localhost:6420)
    --token <token>             Rivet token (default: dev)
    --namespace <name>          Rivet namespace (default: default)
    --runner <name>             Runner name selector (default: test-runner)

  Test Variations:
    --variation <type>          Test variation type (default: sporadic)
                                Options:
                                  - sporadic: Create, test, destroy immediately
                                  - idle: Create, test, sleep for duration, destroy
                                  - chatty: Continuously send requests/messages for duration
    --variation-duration <secs> Duration in seconds for idle/chatty variations (default: 120)

  Output Options:
    --out <format>              Output results to external service
                                Examples: json=results.json, influxdb=http://localhost:8086
    --summary-trend-stats <stats> Summary statistics to show (default: avg,min,med,max,p(90),p(95),p(99))
    -q, --quiet                 Suppress progress output during test
    --skip-health-check         Skip health checks before running test

EXAMPLES:
  # Quick sporadic test with 5 users for 1 minute
  tsx run.ts --stages "1m:5"

  # Idle actors test - actors sleep for 5 minutes
  tsx run.ts --variation idle --variation-duration 300 --stages "1m:10"

  # Chatty actors test - continuously send requests for 3 minutes
  tsx run.ts --variation chatty --variation-duration 180 --stages "2m:5"

  # Stress test ramping up to 50 users
  tsx run.ts --stages "2m:10,5m:50,2m:0"

  # Constant load test with 20 users for 5 minutes
  tsx run.ts --executor constant-vus --stages "5m:20"

  # Save results to JSON file
  tsx run.ts --stages "1m:10" --out json=results.json

  # Test against custom endpoint
  tsx run.ts --endpoint https://api.rivet.dev --token <your-token>

METRICS:
  The test tracks custom metrics including:
  - actor_create_success: Success rate of actor creation
  - actor_destroy_success: Success rate of actor destruction
  - actor_ping_success: Success rate of HTTP ping requests
  - actor_sleep_success: Success rate of sleep operations
  - actor_wake_success: Success rate of wake operations
  - websocket_success: Success rate of WebSocket connections
  - actor_create_duration: Time to create actors
  - actor_destroy_duration: Time to destroy actors
  - websocket_message_duration: WebSocket message round-trip time
  - chatty_requests_sent: Total HTTP requests sent (chatty variation only)
  - chatty_websocket_messages_sent: Total WebSocket messages sent (chatty variation only)

THRESHOLDS:
  Default thresholds (test fails if not met):
  - Actor operations: 95% success rate
  - WebSocket: 90% success rate
  - HTTP requests: p95 < 5s, p99 < 10s
  - Actor creation: p95 < 3s

For more information about k6 executors and options:
  https://grafana.com/docs/k6/latest/using-k6/scenarios/
`);
}

// Check if k6 is installed
async function checkK6Installed(): Promise<boolean> {
	return new Promise((resolve) => {
		const k6Check = spawn("k6", ["version"], { stdio: "pipe" });
		k6Check.on("close", (code) => {
			resolve(code === 0);
		});
		k6Check.on("error", () => {
			resolve(false);
		});
	});
}

// Check if the Rivet engine is reachable
async function checkEngineHealth(endpoint: string): Promise<boolean> {
	try {
		const controller = new AbortController();
		const timeoutId = setTimeout(() => controller.abort(), 5000);

		const response = await fetch(`${endpoint}/metadata`, {
			method: "GET",
			signal: controller.signal,
		});

		clearTimeout(timeoutId);

		if (!response.ok) {
			return false;
		}

		// We just care if we can reach it, any response (even errors) is fine
		return true;
	} catch (error) {
		return false;
	}
}

// Check if test-runner is healthy and ready
async function checkTestRunnerHealth(
	endpoint: string,
	namespace: string,
	runnerName: string,
): Promise<boolean> {
	try {
		const response = await fetch(
			`${endpoint}/runner-configs?runner_name=${runnerName}&namespace=${namespace}`,
		);

		if (!response.ok) {
			return false;
		}

		const data = (await response.json()) as {
			runner_configs: Record<string, unknown>;
		};

		// Check if runner config exists
		if (Object.keys(data.runner_configs).length == 0) {
			return false;
		}

		return true;
	} catch (error) {
		return false;
	}
}

// Print help message when engine is not reachable
function printEngineHelp(endpoint: string) {
	console.error(`
╔════════════════════════════════════════════════════════════════════════════╗
║                      Rivet Engine Not Reachable                            ║
╚════════════════════════════════════════════════════════════════════════════╝

Cannot connect to the Rivet engine at: ${endpoint}

Common solutions:

1. Start the Rivet engine locally:
   cd docker/dev
   docker-compose up -d

2. Check if the engine is running:
   curl ${endpoint}/metadata

3. Verify the endpoint URL is correct:
   Current: ${endpoint}
   Default: http://localhost:6420

   Use --endpoint flag to specify a different URL:
   tsx run.ts --endpoint http://your-engine:6420 --stages "1m:5"
`);
}

// Print help message for starting test-runner
function printTestRunnerHelp() {
	console.error(`
╔════════════════════════════════════════════════════════════════════════════╗
║                         Test Runner Not Running                            ║
╚════════════════════════════════════════════════════════════════════════════╝

The Rivet test runner is required to execute actor lifecycle tests.

To start the test runner:

  cd engine/sdks/typescript/test-runner
  pnpm install  # if not already installed
  pnpm start

The test runner will:
  1. Connect to the Rivet engine at ${RIVET_ENDPOINT}
  2. Register itself as a runner named "${RUNNER_NAME_SELECTOR}"
  3. Handle actor lifecycle operations (create, start, stop, destroy)
  4. Provide HTTP and WebSocket endpoints for actors
`);
}

// Run the k6 load test
function runK6Test(config: LoadTestConfig): Promise<number> {
	return new Promise((resolve, reject) => {
		const scriptPath = new URL("./actor-lifecycle.js", import.meta.url)
			.pathname;

		const env = {
			...process.env,
			EXECUTOR: config.executor,
			START_VUS: config.startVUs.toString(),
			STAGES: config.stages,
			GRACEFUL_RAMPDOWN: config.gracefulRampDown,
			RIVET_ENDPOINT: config.rivetEndpoint,
			RIVET_TOKEN: config.rivetToken,
			RIVET_NAMESPACE: config.rivetNamespace,
			RUNNER_NAME_SELECTOR: config.runnerNameSelector,
			VARIATION: config.variation,
			VARIATION_DURATION: config.variationDuration.toString(),
			K6_SUMMARY_TREND_STATS: config.summaryTrendStats,
		};

		const args = ["run"];

		if (config.out) {
			args.push("--out", config.out);
		}

		if (config.quiet) {
			args.push("--quiet");
		}

		args.push(scriptPath);

		console.log("\n🚀 Starting k6 load test...\n");
		console.log("Configuration:");
		console.log(`  Executor: ${config.executor}`);
		console.log(`  Stages: ${config.stages}`);
		console.log(`  Variation: ${config.variation}`);
		if (config.variation !== "sporadic") {
			console.log(`  Variation Duration: ${config.variationDuration}s`);
		}
		console.log(`  Endpoint: ${config.rivetEndpoint}`);
		console.log(`  Namespace: ${config.rivetNamespace}`);
		console.log(`  Runner: ${config.runnerNameSelector}`);
		console.log("");

		const k6Process = spawn("k6", args, {
			env,
			stdio: "inherit",
		});

		k6Process.on("close", (code) => {
			resolve(code ?? 0);
		});

		k6Process.on("error", (error) => {
			reject(error);
		});

		// Handle graceful shutdown
		process.on("SIGINT", () => {
			console.log("\n\nReceived SIGINT, stopping k6...");
			k6Process.kill("SIGINT");
		});

		process.on("SIGTERM", () => {
			console.log("\n\nReceived SIGTERM, stopping k6...");
			k6Process.kill("SIGTERM");
		});
	});
}

async function main() {
	const config = parseArguments();

	// Check if k6 is installed
	console.log("Checking k6 installation...");
	const k6Installed = await checkK6Installed();

	if (!k6Installed) {
		console.error(`
❌ Error: k6 is not installed or not in PATH

To install k6, visit: https://k6.io/docs/get-started/installation/

Quick install options:
  macOS:   brew install k6
  Linux:   sudo gpg -k
           sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
           echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
           sudo apt-get update
           sudo apt-get install k6
  Windows: choco install k6
  Docker:  docker pull grafana/k6
`);
		process.exit(1);
	}

	console.log("✓ k6 is installed\n");

	// Health checks
	if (!config.skipHealthCheck) {
		// First check if engine is reachable
		console.log("Checking Rivet engine health...");
		const engineHealthy = await checkEngineHealth(config.rivetEndpoint);

		if (!engineHealthy) {
			printEngineHelp(config.rivetEndpoint);
			process.exit(1);
		}

		console.log("✓ Rivet engine is reachable\n");

		// Then check if test runner is configured
		console.log("Checking test runner health...");
		const runnerHealthy = await checkTestRunnerHealth(
			config.rivetEndpoint,
			config.rivetNamespace,
			config.runnerNameSelector,
		);

		if (!runnerHealthy) {
			printTestRunnerHelp();
			process.exit(1);
		}

		console.log("✓ Test runner is healthy\n");
	}

	// Run the k6 test
	try {
		const exitCode = await runK6Test(config);
		process.exit(exitCode);
	} catch (error) {
		console.error("❌ Error running k6 test:", error);
		process.exit(1);
	}
}

main().catch((error) => {
	console.error("Fatal error:", error);
	process.exit(1);
});
