import http from 'k6/http';
import ws from 'k6/ws';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';
import encoding from 'k6/encoding';
import exec from 'k6/execution';

// Custom metrics for detailed tracking
const actorCreateSuccessRate = new Rate('actor_create_success');
const actorDestroySuccessRate = new Rate('actor_destroy_success');
const actorPingSuccessRate = new Rate('actor_ping_success');
const actorSleepSuccessRate = new Rate('actor_sleep_success');
const actorWakeSuccessRate = new Rate('actor_wake_success');
const websocketSuccessRate = new Rate('websocket_success');

const actorCreateDuration = new Trend('actor_create_duration');
const actorDestroyDuration = new Trend('actor_destroy_duration');
const websocketMessageDuration = new Trend('websocket_message_duration');

const activeActors = new Counter('active_actors_count');
const chattyRequestsCount = new Counter('chatty_requests_sent');
const chattyMessagesCount = new Counter('chatty_websocket_messages_sent');

// Get environment variables with defaults
const RIVET_ENDPOINT = __ENV.RIVET_ENDPOINT || 'http://localhost:6420';
const RIVET_TOKEN = __ENV.RIVET_TOKEN || 'dev';
const RIVET_NAMESPACE = __ENV.RIVET_NAMESPACE || 'default';
const RUNNER_NAME_SELECTOR = __ENV.RUNNER_NAME_SELECTOR || 'test-runner';
const VARIATION = __ENV.VARIATION || 'sporadic'; // sporadic, idle, chatty
const VARIATION_DURATION = parseInt(__ENV.VARIATION_DURATION || '120'); // seconds

// Calculate total stage duration for ramp-down detection
function calculateStageDuration(stagesStr) {
	const stages = stagesStr.split(',');
	let totalSeconds = 0;

	for (const stage of stages) {
		const [duration] = stage.split(':');
		totalSeconds += parseDuration(duration);
	}

	return totalSeconds;
}

// Parse duration string (e.g., "1m", "30s", "1h30m") to seconds
function parseDuration(duration) {
	let totalSeconds = 0;
	let currentNumber = '';

	for (let i = 0; i < duration.length; i++) {
		const char = duration[i];

		if (char >= '0' && char <= '9') {
			currentNumber += char;
		} else if (char === 'h') {
			totalSeconds += parseInt(currentNumber || '0') * 60 * 60;
			currentNumber = '';
		} else if (char === 'm') {
			totalSeconds += parseInt(currentNumber || '0') * 60;
			currentNumber = '';
		} else if (char === 's') {
			totalSeconds += parseInt(currentNumber || '0');
			currentNumber = '';
		}
	}

	// If there's a trailing number without unit, assume seconds
	if (currentNumber) {
		totalSeconds += parseInt(currentNumber);
	}

	return totalSeconds;
}

// Test configuration via environment variables
export const options = {
	scenarios: {
		actor_lifecycle: {
			executor: __ENV.EXECUTOR || 'ramping-vus',
			startVUs: parseInt(__ENV.START_VUS || '0'),
			stages: parseStages(__ENV.STAGES || '1m:10,2m:20,1m:0'),
			gracefulRampDown: __ENV.GRACEFUL_RAMPDOWN || '30s',
		},
	},
	thresholds: {
		'actor_create_success': ['rate>0.95'],
		'actor_destroy_success': ['rate>0.95'],
		'actor_ping_success': ['rate>0.95'],
		'websocket_success': ['rate>0.90'],
	},
	noConnectionReuse: false,
	userAgent: 'k6-actor-lifecycle-test',
};

// Parse stages from string format: "1m:10,2m:20,1m:0"
function parseStages(stagesStr) {
	return stagesStr.split(',').map(stage => {
		const [duration, target] = stage.split(':');
		return { duration, target: parseInt(target) };
	});
}

// Generate a unique actor key for this VU and iteration
function generateActorKey() {
	return `load-test-${__VU}-${__ITER}-${Date.now()}`;
}

// Create an actor
function createActor() {
	const key = generateActorKey();
	const startTime = Date.now();

	const payload = JSON.stringify({
		name: 'load-test-actor',
		key,
		input: encoding.b64encode('load-test'),
		runner_name_selector: RUNNER_NAME_SELECTOR,
		crash_policy: 'sleep',
	});

	let response;
	try {
		response = http.post(
			`${RIVET_ENDPOINT}/actors?namespace=${RIVET_NAMESPACE}`,
			payload,
			{
				headers: {
					'Authorization': `Bearer ${RIVET_TOKEN}`,
					'Content-Type': 'application/json',
				},
				tags: { name: 'create_actor' },
			}
		);
	} catch (error) {
		console.error(`[CreateActor] Request failed: ${error}`);
		actorCreateSuccessRate.add(false);
		return null;
	}

	const duration = Date.now() - startTime;
	actorCreateDuration.add(duration);

	const success = check(response, {
		'actor created': (r) => r.status === 200,
		'actor has id': (r) => {
			try {
				const body = JSON.parse(r.body);
				return body.actor && body.actor.actor_id;
			} catch (parseError) {
				console.error(`[CreateActor] Failed to parse response: ${parseError}`);
				return false;
			}
		},
	});

	actorCreateSuccessRate.add(success);

	if (!success) {
		console.error(`[CreateActor] Failed with status ${response.status}: ${response.body}`);
		return null;
	}

	const body = JSON.parse(response.body);
	activeActors.add(1);

	return {
		actorId: body.actor.actor_id,
		key,
	};
}

// Ping the actor via HTTP
function pingActor(actorId) {
	let response;
	try {
		response = http.get(
			`${RIVET_ENDPOINT}/ping`,
			{
				headers: {
					'X-Rivet-Token': RIVET_TOKEN,
					'X-Rivet-Target': 'actor',
					'X-Rivet-Actor': actorId,
				},
				tags: { name: 'ping_actor' },
			}
		);
	} catch (error) {
		console.error(`[PingActor ${actorId}] Request failed: ${error}`);
		actorPingSuccessRate.add(false);
		return false;
	}

	const success = check(response, {
		'ping successful': (r) => r.status === 200,
		'ping has response': (r) => r.body && r.body.length > 0,
	});

	actorPingSuccessRate.add(success);

	if (!success) {
		console.error(`[PingActor ${actorId}] Failed with status ${response.status}: ${response.body}`);
	}

	return success;
}

// Test WebSocket connection to actor
function testWebSocket(actorId) {
	const wsEndpoint = RIVET_ENDPOINT.replace('http://', 'ws://').replace('https://', 'wss://');
	const wsUrl = `${wsEndpoint}/gateway/${actorId}@${RIVET_TOKEN}/ws`;

	let success = false;
	let messagesReceived = 0;
	let messageSentAt = 0;

	let response;
	try {
		response = ws.connect(wsUrl, (socket) => {
			socket.on('open', () => {
				try {
					// Send a ping message
					messageSentAt = Date.now();
					socket.send('ping');
				} catch (error) {
					console.error(`[WebSocket ${actorId}] Failed to send ping: ${error}`);
					socket.close();
				}
			});

			socket.on('message', (data) => {
				messagesReceived++;
				if (messageSentAt > 0) {
					const duration = Date.now() - messageSentAt;
					websocketMessageDuration.add(duration);
				}

				const message = data.toString();

				if (message === 'Echo: ping' || message === 'pong') {
					try {
						// Send hello message
						messageSentAt = Date.now();
						socket.send('hello');
					} catch (error) {
						console.error(`[WebSocket ${actorId}] Failed to send hello: ${error}`);
						socket.close();
					}
				} else if (message === 'Echo: hello') {
					success = true;
					socket.close();
				}
			});

			socket.on('error', (e) => {
				console.error(`[WebSocket ${actorId}] Socket error: ${e}`);
				socket.close();
			});

			// Set timeout to close connection if messages take too long
			socket.setTimeout(() => {
				if (messagesReceived === 0) {
					console.error(`[WebSocket ${actorId}] Timeout - no messages received`);
				}
				socket.close();
			}, 5000);
		});
	} catch (error) {
		console.error(`[WebSocket ${actorId}] Connection failed: ${error}`);
		websocketSuccessRate.add(false);
		return false;
	}

	const connected = check(response, {
		'websocket connected': (r) => r && r.status === 101,
	});

	if (!connected) {
		console.error(`[WebSocket ${actorId}] Failed to connect - status: ${response?.status || 'unknown'}`);
	}

	websocketSuccessRate.add(success);
	return success;
}

// Keep WebSocket open during idle period without sending messages
function idleWebSocket(actorId, durationSeconds) {
	const wsEndpoint = RIVET_ENDPOINT.replace('http://', 'ws://').replace('https://', 'wss://');
	const wsUrl = `${wsEndpoint}/gateway/${actorId}@${RIVET_TOKEN}/ws`;

	let success = false;
	let closedEarly = false;

	// Calculate how long until ramp-down starts
	const stageDuration = calculateStageDuration(__ENV.STAGES || '1m:10,2m:20,1m:0');
	const elapsed = (Date.now() - exec.scenario.startTime) / 1000;
	const timeUntilRampDown = Math.max(0, stageDuration - elapsed);

	// Use the shorter of: requested duration or time until ramp-down
	const actualDuration = Math.min(durationSeconds, timeUntilRampDown);
	const durationMs = actualDuration * 1000;

	if (actualDuration < durationSeconds) {
		console.log(`[IdleWS ${actorId}] Limiting idle to ${Math.floor(actualDuration)}s (ramp-down starts soon)`);
	}

	try {
		const response = ws.connect(wsUrl, (socket) => {
			socket.on('open', () => {
				console.log(`[IdleWS ${actorId}] Connection opened, keeping idle for ${Math.floor(actualDuration)}s`);
				success = true;
			});

			socket.on('message', (data) => {
				// Log any messages received from the server during idle period
				console.log(`[IdleWS ${actorId}] Received: ${data.toString()}`);
			});

			socket.on('error', (e) => {
				console.error(`[IdleWS ${actorId}] Socket error: ${e}`);
				closedEarly = true;
			});

			socket.on('close', () => {
				if (closedEarly) {
					console.warn(`[IdleWS ${actorId}] Connection closed early`);
				} else {
					console.log(`[IdleWS ${actorId}] Connection closed after ${Math.floor(actualDuration)}s`);
				}
			});

			// Set timeout to close the connection after the actual duration
			socket.setTimeout(() => {
				console.log(`[IdleWS ${actorId}] Idle period complete, closing connection`);
				socket.close();
			}, durationMs);
		});

		websocketSuccessRate.add(success);
	} catch (error) {
		console.error(`[IdleWS ${actorId}] Connection failed: ${error}`);
		websocketSuccessRate.add(false);
	}

	return success;
}

// Sleep the actor
function sleepActor(actorId) {
	let response;
	try {
		response = http.get(
			`${RIVET_ENDPOINT}/sleep`,
			{
				headers: {
					'X-Rivet-Token': RIVET_TOKEN,
					'X-Rivet-Target': 'actor',
					'X-Rivet-Actor': actorId,
				},
				tags: { name: 'sleep_actor' },
			}
		);
	} catch (error) {
		console.error(`[SleepActor ${actorId}] Request failed: ${error}`);
		actorSleepSuccessRate.add(false);
		return false;
	}

	const success = check(response, {
		'sleep successful': (r) => r.status === 200,
	});

	actorSleepSuccessRate.add(success);

	if (!success) {
		console.error(`[SleepActor ${actorId}] Failed with status ${response.status}: ${response.body}`);
	}

	return success;
}

// Wake the actor with a ping request
function wakeActor(actorId) {
	let response;
	try {
		response = http.get(
			`${RIVET_ENDPOINT}/ping`,
			{
				headers: {
					'X-Rivet-Token': RIVET_TOKEN,
					'X-Rivet-Target': 'actor',
					'X-Rivet-Actor': actorId,
				},
				tags: { name: 'wake_actor' },
			}
		);
	} catch (error) {
		console.error(`[WakeActor ${actorId}] Request failed: ${error}`);
		actorWakeSuccessRate.add(false);
		return false;
	}

	const success = check(response, {
		'wake successful': (r) => r.status === 200,
	});

	actorWakeSuccessRate.add(success);

	if (!success) {
		console.error(`[WakeActor ${actorId}] Failed with status ${response.status}: ${response.body}`);
	}

	return success;
}

// Destroy the actor
function destroyActor(actorId) {
	const startTime = Date.now();

	let response;
	try {
		response = http.del(
			`${RIVET_ENDPOINT}/actors/${actorId}?namespace=${RIVET_NAMESPACE}`,
			null,
			{
				headers: {
					'Authorization': `Bearer ${RIVET_TOKEN}`,
				},
				tags: { name: 'destroy_actor' },
			}
		);
	} catch (error) {
		console.error(`[DestroyActor ${actorId}] Request failed: ${error}`);
		actorDestroySuccessRate.add(false);
		return false;
	}

	const duration = Date.now() - startTime;
	actorDestroyDuration.add(duration);

	const success = check(response, {
		'actor destroyed': (r) => r.status === 200,
	});

	actorDestroySuccessRate.add(success);

	if (!success) {
		console.error(`[DestroyActor ${actorId}] Failed with status ${response.status}: ${response.body}`);
	}

	if (success) {
		activeActors.add(-1);
	}

	return success;
}

// Main test function - executed by each VU
export default function () {
	if (VARIATION === 'sporadic') {
		runSporadicTest();
	} else if (VARIATION === 'idle') {
		runIdleTest();
	} else if (VARIATION === 'chatty') {
		runChattyTest();
	} else {
		console.error(`Unknown variation: ${VARIATION}`);
	}
}

// Sporadic test - create, test, destroy immediately (original behavior)
function runSporadicTest() {
	let actorId = null;

	try {
		// 1. Create an actor
		const actor = createActor();
		if (!actor) {
			return;
		}
		actorId = actor.actorId;

		console.log(`[Sporadic] Created actor ${actor.key} ${actorId}`);

		// Small delay to let actor fully initialize
		sleep(0.5);

		// 2. Ping the actor via HTTP
		if (!pingActor(actorId)) {
			console.warn(`Ping failed for actor ${actorId}`);
		}

		// Small delay between operations
		sleep(0.2);

		// 3. Test WebSocket connection
		if (!testWebSocket(actorId)) {
			console.warn(`WebSocket test failed for actor ${actorId}`);
		}

		// Small delay between operations
		sleep(0.2);

		// 4. Sleep the actor
		if (!sleepActor(actorId)) {
			console.warn(`Sleep failed for actor ${actorId}`);
		}

		// Wait a bit while actor is sleeping
		sleep(1);

		// 5. Wake the actor with a ping
		if (!wakeActor(actorId)) {
			console.warn(`Wake failed for actor ${actorId}`);
		}

		// Small delay before destruction
		sleep(0.2);

		// 6. Destroy the actor
		if (!destroyActor(actorId)) {
			console.error(`Failed to destroy actor ${actorId}`);
		}
		actorId = null;

	} catch (error) {
		console.error(`Error in sporadic test: ${error}`);
	} finally {
		if (actorId) {
			destroyActor(actorId);
		}
	}

	sleep(1);
}

// Idle test - create, test, keep WebSocket open during sleep, then destroy
function runIdleTest() {
	let actorId = null;

	try {
		// 1. Create an actor
		const actor = createActor();
		if (!actor) {
			return;
		}
		actorId = actor.actorId;

		console.log(`[Idle] Created actor ${actor.key} ${actorId}`);

		// Small delay to let actor fully initialize
		sleep(0.5);

		// 2. Basic lifecycle test - initial ping
		pingActor(actorId);
		sleep(0.2);

		// 3. Sleep the actor
		if (!sleepActor(actorId)) {
			console.warn(`Sleep failed for actor ${actorId}`);
		}

		// Calculate how long until ramp-down starts
		const stageDuration = calculateStageDuration(__ENV.STAGES || '1m:10,2m:20,1m:0');
		const elapsed = (Date.now() - exec.scenario.startTime) / 1000;
		const timeUntilRampDown = Math.max(0, stageDuration - elapsed);

		// Use the shorter of: requested duration or time until ramp-down
		const actualDuration = Math.min(VARIATION_DURATION, timeUntilRampDown);

		if (actualDuration === 0) {
			console.log(`[Idle] Test in graceful ramp-down period, skipping idle period for graceful shutdown`);
		} else {
			if (actualDuration < VARIATION_DURATION) {
				console.log(`[Idle] Limiting idle to ${Math.floor(actualDuration)}s (ramp-down starts soon)`);
			} else {
				console.log(`[Idle] Actor ${actorId} sleeping for ${VARIATION_DURATION}s with WebSocket kept alive`);
			}

			// 4. Keep WebSocket open during the actual idle duration
			idleWebSocket(actorId, actualDuration);
		}

		// 5. Wake and destroy
		wakeActor(actorId);
		sleep(0.5);

		if (!destroyActor(actorId)) {
			console.error(`Failed to destroy idle actor ${actorId}`);
		}
		actorId = null;

	} catch (error) {
		console.error(`Error in idle test: ${error}`);
	} finally {
		if (actorId) {
			destroyActor(actorId);
		}
	}

	sleep(1);
}

// Chatty test - create, continuously send requests and websocket messages for duration
function runChattyTest() {
	let actorId = null;

	try {
		// 1. Create an actor
		const actor = createActor();
		if (!actor) {
			return;
		}
		actorId = actor.actorId;

		console.log(`[Chatty] Created actor ${actor.key} ${actorId}, will be chatty for ${VARIATION_DURATION}s`);

		// Small delay to let actor fully initialize
		sleep(0.5);

		// Calculate how long until ramp-down starts
		const stageDuration = calculateStageDuration(__ENV.STAGES || '1m:10,2m:20,1m:0');
		const elapsed = (Date.now() - exec.scenario.startTime) / 1000;
		const timeUntilRampDown = Math.max(0, stageDuration - elapsed);

		// Use the shorter of: requested duration or time until ramp-down
		const actualDuration = Math.min(VARIATION_DURATION, timeUntilRampDown);

		if (actualDuration === 0) {
			console.log(`[Chatty] Test in graceful ramp-down period, skipping chatty period for graceful shutdown`);
		} else {
			if (actualDuration < VARIATION_DURATION) {
				console.log(`[Chatty] Limiting chatty to ${Math.floor(actualDuration)}s (ramp-down starts soon)`);
			}

			// 2. Be chatty for the actual duration
			const endTime = Date.now() + (actualDuration * 1000);
			let requestCount = 0;

			// Start a WebSocket connection and keep it open, sending messages
			// We'll run this in parallel with HTTP requests
			const wsEndpoint = RIVET_ENDPOINT.replace('http://', 'ws://').replace('https://', 'wss://');
			const wsUrl = `${wsEndpoint}/gateway/${actorId}@${RIVET_TOKEN}/ws`;

			// For chatty mode, we alternate between HTTP and WebSocket
			while (Date.now() < endTime) {

				// Send HTTP ping
				try {
					const pingResponse = http.get(
						`${RIVET_ENDPOINT}/ping`,
						{
							headers: {
								'X-Rivet-Token': RIVET_TOKEN,
								'X-Rivet-Target': 'actor',
								'X-Rivet-Actor': actorId,
							},
							tags: { name: 'chatty_ping' },
						}
					);

					if (pingResponse.status === 200) {
						chattyRequestsCount.add(1);
						requestCount++;
					} else {
						console.error(`[Chatty ${actorId}] Ping failed with status ${pingResponse.status}: ${pingResponse.body}`);
					}
				} catch (error) {
					console.error(`[Chatty ${actorId}] Ping request failed: ${error}`);
				}

				// Send a few WebSocket messages
				try {
					ws.connect(wsUrl, (socket) => {
						socket.on('open', () => {
							try {
								for (let i = 0; i < 3; i++) {
									socket.send(`chatty-msg-${requestCount}-${i}`);
									chattyMessagesCount.add(1);
								}
								socket.close();
							} catch (error) {
								console.error(`[Chatty ${actorId}] Failed to send WS messages: ${error}`);
								socket.close();
							}
						});

						socket.on('error', (e) => {
							console.error(`[Chatty ${actorId}] WS error: ${e}`);
							socket.close();
						});

						socket.setTimeout(() => {
							socket.close();
						}, 1000);
					});
				} catch (error) {
					console.error(`[Chatty ${actorId}] WS connection failed: ${error}`);
				}

				// Small delay between bursts
				sleep(0.5);
			}

			console.log(`[Chatty] Actor ${actorId} sent ${requestCount} HTTP requests`);
		}

		// 3. Destroy the actor
		if (!destroyActor(actorId)) {
			console.error(`Failed to destroy chatty actor ${actorId}`);
		}
		actorId = null;

	} catch (error) {
		console.error(`Error in chatty test: ${error}`);
	} finally {
		if (actorId) {
			destroyActor(actorId);
		}
	}

	sleep(1);
}

// Teardown function - runs once at the end
export function teardown(data) {
	console.log('Load test completed');
}
