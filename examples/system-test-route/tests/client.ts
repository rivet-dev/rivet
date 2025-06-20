//import { RivetClient } from "@rivet-gg/api-full";
//import crypto from "crypto";
//import http from "http";
//
//// Can be opt since they're not required for dev
//const RIVET_ENDPOINT = process.env.RIVET_ENDPOINT;
//const RIVET_SERVICE_TOKEN = process.env.RIVET_SERVICE_TOKEN;
//const RIVET_PROJECT = process.env.RIVET_PROJECT;
//const RIVET_ENVIRONMENT = process.env.RIVET_ENVIRONMENT;
//
//// Determine test kind from environment variable
//const BUILD_NAME = process.env.BUILD;
//if (BUILD_NAME !== "http-isolate" && BUILD_NAME !== "http-container") {
//	throw new Error(
//		"Must specify BUILD environment variable as either 'http-isolate' or 'http-container'",
//	);
//}
//
//let region = process.env.REGION;
//if (!region || region.length === 0) {
//	region = undefined;
//}
//
//const client = new RivetClient({
//	environment: RIVET_ENDPOINT,
//	token: RIVET_SERVICE_TOKEN,
//});
//
//// Interface definitions
//interface RouteConfig {
//	path: string;
//	routeSubpaths: boolean;
//	stripPrefix?: boolean; // Whether to strip the prefix from the path in request handlers
//	selectorIndex?: number; // Index of the selector to use from selectors array
//}
//
//interface TestContext {
//	actorIds: string[];
//	routeIds: string[];
//	selectors: string[]; // Array of selectors
//	hostname: string;
//	actorsBySelector: Record<string, string[]>; // Maps selectors to their actor IDs
//	routes?: RouteConfig[]; // Store the routes for this test
//}
//
//interface TestConfig {
//	name: string;
//	numActors?: number;
//	numSelectors?: number; // Number of different selectors to create
//	routes?: RouteConfig[]; // Each route can specify which selector to use
//}
//
//// Helper function to make HTTP requests with a custom host header
//async function makeRequest(url: string, hostname: string): Promise<any> {
//	return new Promise<any>((resolve, reject) => {
//		const parsedUrl = new URL(url);
//
//		const options = {
//			hostname: parsedUrl.hostname,
//			port: parsedUrl.port || 80,
//			path: parsedUrl.pathname + parsedUrl.search,
//			method: "GET",
//			headers: {
//				Accept: "application/json",
//				Host: hostname,
//			},
//		};
//
//		const req = http.request(options, (res: any) => {
//			if (res.statusCode !== 200) {
//				console.error(
//					`Request failed: ${res.statusCode} ${res.statusMessage}`,
//				);
//				// Don't reject, just continue the loop
//				resolve(null);
//				return;
//			}
//
//			let rawData = "";
//			res.on("data", (chunk: any) => {
//				rawData += chunk;
//			});
//			res.on("end", () => {
//				try {
//					const parsedData = JSON.parse(rawData);
//					resolve(parsedData);
//				} catch (e) {
//					console.error("Error parsing response:", e);
//					reject(e);
//				}
//			});
//		});
//
//		req.on("error", (e: any) => {
//			console.error(`Request error: ${e.message}`);
//			reject(e);
//		});
//
//		req.end();
//	});
//}
//
//// Helper function to create actors with a specific selector
//async function createActors(
//	selectorTag: string,
//	numberOfActors: number = 2,
//): Promise<string[]> {
//	const createdActorIds: string[] = [];
//
//	for (let i = 1; i <= numberOfActors; i++) {
//		console.time(`create actor ${i}`);
//		console.log(`Creating actor ${i} with tag`, {
//			selector: selectorTag,
//		});
//
//		const { actor } = await client.actors.create({
//			project: RIVET_PROJECT,
//			environment: RIVET_ENVIRONMENT,
//			body: {
//				region,
//				tags: {
//					selector: selectorTag,
//					instance: i.toString(),
//				},
//				buildTags: { name: BUILD_NAME, current: "true" },
//				network: {
//					ports: {
//						http: {
//							protocol: "https",
//							routing: {
//								guard: {},
//							},
//						},
//					},
//				},
//				lifecycle: {
//					durable: false,
//				},
//				...(BUILD_NAME === "http-container"
//					? {
//							resources: {
//								cpu: 100,
//								memory: 100,
//							},
//						}
//					: {}),
//			},
//		});
//
//		createdActorIds.push(actor.id);
//		console.timeEnd(`create actor ${i}`);
//		console.log(`Created actor ${i} with ID:`, actor.id);
//	}
//
//	// Wait for actors to be ready
//	await new Promise((resolve) => setTimeout(resolve, 2000));
//	return createdActorIds;
//}
//
//// Helper function to create a route
//async function createRoute(
//	routeId: string,
//	hostname: string,
//	selectorTag: string,
//	path: string,
//	routeSubpaths: boolean = false,
//	stripPrefix: boolean = true,
//): Promise<void> {
//	console.time(`create route ${routeId}`);
//	console.log(`Creating route ${routeId} with selector tag`, {
//		selector: selectorTag,
//		path,
//		routeSubpaths,
//		stripPrefix,
//	});
//
//	await client.routes.update(routeId, {
//		project: RIVET_PROJECT,
//		environment: RIVET_ENVIRONMENT,
//		body: {
//			hostname,
//			path,
//			routeSubpaths,
//			stripPrefix,
//			target: {
//				actors: {
//					selectorTags: {
//						selector: selectorTag,
//					},
//				},
//			},
//		},
//	});
//
//	console.timeEnd(`create route ${routeId}`);
//	console.log(`Created route ${routeId}.`);
//
//	// Wait for route to be active
//	await new Promise((resolve) => setTimeout(resolve, 2000));
//}
//
//// Helper function to calculate expected path based on route configuration
//function getExpectedPath(
//	requestPath: string,
//	routePath: string,
//	stripPrefix: boolean,
//): string {
//	// Extract the query string if present
//	const queryStringIndex = requestPath.indexOf("?");
//	const pathWithoutQuery =
//		queryStringIndex >= 0
//			? requestPath.substring(0, queryStringIndex)
//			: requestPath;
//
//	// If stripPrefix is false, the full path should be returned
//	if (!stripPrefix) {
//		return pathWithoutQuery;
//	}
//
//	// If stripPrefix is true, we need to strip the route path prefix
//	if (routePath === "") {
//		// For empty path routes, return the path as is
//		return pathWithoutQuery;
//	}
//
//	// For non-empty paths with stripPrefix=true
//	if (pathWithoutQuery === routePath) {
//		// If exact match, return "/"
//		return "/";
//	} else if (pathWithoutQuery.startsWith(routePath + "/")) {
//		// If it's a subpath, remove the prefix
//		return pathWithoutQuery.substring(routePath.length);
//	}
//
//	// Default case - shouldn't happen with proper routing
//	return pathWithoutQuery;
//}
//
//// Helper function to test a route
//async function testRoute(
//	hostname: string,
//	path: string,
//	numActorsExpected: number = 2,
//	maxRequests: number = 20,
//	route?: RouteConfig, // Added route config parameter
//): Promise<Set<string>> {
//	const actorIds = new Set<string>();
//	let successfulMatches = 0;
//	let totalRequests = 0;
//	let pathValidationFailed = false;
//
//	// Using localhost with Host header for local testing
//	const testUrl = `http://localhost:7080${path}`;
//	console.log(`Testing route at: ${testUrl} (with Host: ${hostname})`);
//	console.time(`route-test-${path}`);
//
//	// Calculate expected path if route is provided
//	let expectedPath = route
//		? getExpectedPath(
//				path,
//				route.path,
//				route.stripPrefix !== undefined ? route.stripPrefix : true,
//			)
//		: path;
//
//	// URL-encoded characters like %20 will be decoded by the server
//	// Decode the expected path to match server behavior
//	expectedPath = decodeURIComponent(expectedPath);
//
//	if (route) {
//		console.log(
//			`Route config: path=${route.path}, routeSubpaths=${route.routeSubpaths}, stripPrefix=${route.stripPrefix}`,
//		);
//		console.log(`Expected path in response: ${expectedPath}`);
//	}
//
//	while (actorIds.size < numActorsExpected && totalRequests < maxRequests) {
//		totalRequests++;
//
//		try {
//			const data = await makeRequest(testUrl, hostname);
//
//			// If request failed or returned null, continue to next iteration
//			if (!data) {
//				continue;
//			}
//
//			console.log(
//				`Request ${totalRequests}: Response from actor ${data.actorId} with path ${data.path}`,
//			);
//
//			// Validate the path in the response matches the expected path
//			if (data.path !== expectedPath) {
//				console.error(
//					`❌ Path validation failed: Expected ${expectedPath}, got ${data.path}`,
//				);
//				pathValidationFailed = true;
//			} else {
//				console.log(`✅ Path validation passed: ${data.path}`);
//			}
//
//			// Log query parameters if present
//			if (data.query && Object.keys(data.query).length > 0) {
//				console.log(`Query parameters received:`, data.query);
//			}
//
//			// Track the actor IDs we've seen
//			if (data.actorId) {
//				actorIds.add(data.actorId);
//				successfulMatches++;
//			}
//
//			// If we've found all expected actors, we're done
//			if (actorIds.size === numActorsExpected) {
//				console.log(
//					`Successfully received responses from all ${numActorsExpected} actors!`,
//				);
//				break;
//			}
//
//			// Small delay between requests
//			await new Promise((resolve) => setTimeout(resolve, 200));
//		} catch (error) {
//			console.error("Error making request:", error);
//			// Wait a bit longer if there's an error
//			await new Promise((resolve) => setTimeout(resolve, 500));
//		}
//	}
//
//	console.timeEnd(`route-test-${path}`);
//	console.log(
//		`Test completed. Matched ${actorIds.size}/${numActorsExpected} actors in ${totalRequests} requests.`,
//	);
//	console.log(`Actors matched: ${Array.from(actorIds).join(", ")}`);
//
//	if (actorIds.size < numActorsExpected) {
//		console.error(
//			`Failed to reach all ${numActorsExpected} actors through the route!`,
//		);
//	}
//
//	if (pathValidationFailed) {
//		console.error(
//			"Path validation failed: The path in the response did not match the expected path",
//		);
//		throw new Error(`Path validation failed for ${path}`);
//	}
//
//	// Final stats
//	console.log(`
//Route Test Results for ${path}:
//------------------
//Total requests: ${totalRequests}
//Successful responses: ${successfulMatches}
//Unique actors reached: ${actorIds.size}/${numActorsExpected}
//Route: ${testUrl} (Host: ${hostname})
//Path validation: ${pathValidationFailed ? "❌ Failed" : "✅ Passed"}
//------------------
//	`);
//
//	return actorIds;
//}
//
//// Helper function to verify routes exist
//async function verifyRouteExists(hostname: string): Promise<boolean> {
//	console.time("list routes");
//	console.log("Listing routes to verify our route exists");
//	const { routes } = await client.routes.list({
//		project: RIVET_PROJECT,
//		environment: RIVET_ENVIRONMENT,
//	});
//	console.timeEnd("list routes");
//
//	// Find our route in the list
//	const ourRoute = routes.find((route) => route.hostname === hostname);
//	if (!ourRoute) {
//		console.error(
//			`Route with hostname ${hostname} not found in routes list!`,
//		);
//		return false;
//	}
//	console.log("✅ Found our route in the list:", ourRoute);
//	return true;
//}
//
//// Helper function to delete resources
//async function cleanup(context: TestContext): Promise<void> {
//	// Cleanup: delete routes first
//	for (const routeId of context.routeIds) {
//		console.log("Deleting route", routeId);
//		try {
//			await client.routes.delete(routeId, {
//				project: RIVET_PROJECT,
//				environment: RIVET_ENVIRONMENT,
//			});
//			console.log(`Route ${routeId} deleted successfully`);
//		} catch (err) {
//			console.error(`Error deleting route ${routeId}:`, err);
//		}
//	}
//
//	// Then delete all actors
//	for (let i = 0; i < context.actorIds.length; i++) {
//		const actorId = context.actorIds[i];
//		console.log(`Destroying actor ${i + 1}:`, actorId);
//		try {
//			await client.actors.destroy(actorId, {
//				project: RIVET_PROJECT,
//				environment: RIVET_ENVIRONMENT,
//			});
//		} catch (err) {
//			console.error(`Error destroying actor ${i + 1}:`, err);
//		}
//	}
//}
//
//// Core test setup function that handles resource creation and cleanup
//async function setupTest(
//	config: TestConfig,
//	testFn: (context: TestContext) => Promise<void>,
//): Promise<boolean> {
//	console.log(`\n=== ${config.name} ===\n`);
//
//	const baseSelector = `test-${crypto.randomBytes(4).toString("hex")}`;
//	const hostname = `route-${crypto.randomBytes(4).toString("hex")}.rivet-job.local`;
//
//	const context: TestContext = {
//		actorIds: [],
//		routeIds: [],
//		selectors: [],
//		hostname,
//		actorsBySelector: {},
//		routes: config.routes,
//	};
//
//	try {
//		// Create selectors based on config
//		const numSelectors = config.numSelectors || 1;
//
//		// Create selectors and actors for each selector
//		for (let i = 0; i < numSelectors; i++) {
//			const selectorName =
//				numSelectors === 1 ? baseSelector : `${baseSelector}-${i + 1}`;
//			context.selectors.push(selectorName);
//
//			console.log(
//				`Creating actors with selector ${selectorName} (${i + 1}/${numSelectors})`,
//			);
//			const actors = await createActors(
//				selectorName,
//				config.numActors || 2,
//			);
//			context.actorIds.push(...actors);
//			context.actorsBySelector[selectorName] = actors;
//		}
//
//		// Create routes from config
//		if (config.routes && config.routes.length > 0) {
//			for (let i = 0; i < config.routes.length; i++) {
//				const route = config.routes[i];
//				const routeId = `route-${crypto.randomBytes(4).toString("hex")}${i > 0 ? `-${i}` : ""}`;
//
//				// Determine which selector to use for this route
//				const selectorIndex =
//					route.selectorIndex !== undefined ? route.selectorIndex : 0;
//				if (selectorIndex >= context.selectors.length) {
//					throw new Error(
//						`Route ${i} references selector ${selectorIndex} but only ${context.selectors.length} selectors were created`,
//					);
//				}
//
//				const selector = context.selectors[selectorIndex];
//
//				console.log(
//					`Creating route ${routeId} with path ${route.path} using selector ${selector} (index ${selectorIndex})`,
//				);
//				await createRoute(
//					routeId,
//					context.hostname,
//					selector,
//					route.path,
//					route.routeSubpaths,
//					route.stripPrefix !== undefined ? route.stripPrefix : true,
//				);
//				context.routeIds.push(routeId);
//			}
//		}
//
//		// Verify routes exist
//		await verifyRouteExists(context.hostname);
//
//		// Run the test function
//		await testFn(context);
//
//		// If we get here, the test passed
//		console.log(`✅ Test "${config.name}" passed successfully`);
//		return true;
//	} catch (error) {
//		console.error(`❌ Error in ${config.name}:`, error);
//		return false;
//	} finally {
//		// Clean up all resources
//		await cleanup(context);
//	}
//}
//
//// Test implementations
//async function testBasicRoute(): Promise<boolean> {
//	return await setupTest(
//		{
//			name: "Basic Route Test",
//			numSelectors: 1,
//			routes: [
//				{
//					path: "/test",
//					routeSubpaths: false,
//					stripPrefix: true,
//					selectorIndex: 0,
//				},
//			],
//		},
//		async (context) => {
//			// Get the actor IDs for the first selector
//			const selectorActors =
//				context.actorsBySelector[context.selectors[0]];
//			const route = context.routes?.[0];
//
//			// Test the route
//			const result = await testRoute(
//				context.hostname,
//				"/test",
//				2,
//				20,
//				route,
//			);
//			if (result.size < 2) {
//				throw new Error(
//					"Basic route test failed: Could not reach all expected actors",
//				);
//			}
//
//			// Verify we got responses from the correct actors
//			let matchedActors = 0;
//			for (const id of result) {
//				if (selectorActors.includes(id)) {
//					matchedActors++;
//				}
//			}
//
//			if (matchedActors === result.size) {
//				console.log("✅ All requests routed to the correct actors");
//			} else {
//				console.log(
//					`❌ Expected all requests to route to the correct actors, but only ${matchedActors}/${result.size} did`,
//				);
//				throw new Error(
//					`Basic route test failed: ${matchedActors}/${result.size} requests routed to the correct actors`,
//				);
//			}
//		},
//	);
//}
//
//async function testPathPrefix(): Promise<boolean> {
//	return await setupTest(
//		{
//			name: "Path Prefix Test",
//			numSelectors: 1,
//			routes: [
//				{
//					path: "/api",
//					routeSubpaths: true,
//					stripPrefix: true,
//					selectorIndex: 0,
//				},
//			],
//		},
//		async (context) => {
//			// Get the actor IDs for the first selector
//			const selectorActors =
//				context.actorsBySelector[context.selectors[0]];
//			const route = context.routes?.[0];
//
//			// Test various paths that should match the prefix
//			console.log("Testing paths that should match the prefix /api");
//			let result = await testRoute(
//				context.hostname,
//				"/api",
//				2,
//				20,
//				route,
//			);
//			if (result.size < 2) {
//				throw new Error(
//					"Path prefix test failed: Could not reach actors at /api",
//				);
//			}
//
//			result = await testRoute(context.hostname, "/api/v1", 2, 20, route);
//			if (result.size < 2) {
//				throw new Error(
//					"Path prefix test failed: Could not reach actors at /api/v1",
//				);
//			}
//
//			result = await testRoute(
//				context.hostname,
//				"/api/users",
//				2,
//				20,
//				route,
//			);
//			if (result.size < 2) {
//				throw new Error(
//					"Path prefix test failed: Could not reach actors at /api/users",
//				);
//			}
//
//			// Test a path that shouldn't match
//			console.log("Testing path that shouldn't match the prefix /api");
//			const nonMatchingResult = await testRoute(
//				context.hostname,
//				"/other",
//				2,
//				5,
//			);
//
//			if (nonMatchingResult.size > 0) {
//				console.log(
//					"❌ Found match for non-matching path /other when it should have failed",
//				);
//				throw new Error(
//					"Path prefix test failed: Found match for non-matching path /other",
//				);
//			} else {
//				console.log(
//					"✅ Correctly found no matches for non-matching path /other",
//				);
//			}
//		},
//	);
//}
//
//async function testExactPath(): Promise<boolean> {
//	return await setupTest(
//		{
//			name: "Exact Path Test",
//			numSelectors: 1,
//			routes: [
//				{
//					path: "/exact",
//					routeSubpaths: false,
//					stripPrefix: true,
//					selectorIndex: 0,
//				},
//			],
//		},
//		async (context) => {
//			// Get the actor IDs for the selector
//			const selectorActors =
//				context.actorsBySelector[context.selectors[0]];
//			const route = context.routes?.[0];
//
//			// Test the exact path
//			console.log("Testing exact path /exact");
//			const result = await testRoute(
//				context.hostname,
//				"/exact",
//				2,
//				20,
//				route,
//			);
//			if (result.size < 2) {
//				throw new Error(
//					"Exact path test failed: Could not reach actors at exact path /exact",
//				);
//			}
//
//			// Test paths that shouldn't match
//			console.log(
//				"Testing paths that shouldn't match the exact path /exact",
//			);
//			const subPathResult = await testRoute(
//				context.hostname,
//				"/exact/subpath",
//				2,
//				5,
//			);
//
//			if (subPathResult.size > 0) {
//				console.log(
//					"❌ Found match for /exact/subpath when it should have failed",
//				);
//				throw new Error(
//					"Exact path test failed: Found match for subpath /exact/subpath",
//				);
//			} else {
//				console.log("✅ Correctly found no matches for /exact/subpath");
//			}
//
//			const differentPathResult = await testRoute(
//				context.hostname,
//				"/different",
//				2,
//				5,
//			);
//
//			if (differentPathResult.size > 0) {
//				console.log(
//					"❌ Found match for /different when it should have failed",
//				);
//				throw new Error(
//					"Exact path test failed: Found match for different path /different",
//				);
//			} else {
//				console.log("✅ Correctly found no matches for /different");
//			}
//		},
//	);
//}
//
//async function testPathPriority(): Promise<boolean> {
//	return await setupTest(
//		{
//			name: "Path Priority Test",
//			numSelectors: 2,
//			routes: [
//				{
//					path: "/api",
//					routeSubpaths: true,
//					stripPrefix: true,
//					selectorIndex: 0,
//				}, // Less specific path
//				{
//					path: "/api/users",
//					routeSubpaths: true,
//					stripPrefix: true,
//					selectorIndex: 1,
//				}, // More specific path (higher priority)
//			],
//		},
//		async (context) => {
//			// Get the selectors and actor groups
//			const apiSelector = context.selectors[0];
//			const usersSelector = context.selectors[1];
//
//			const apiActors = context.actorsBySelector[apiSelector]; // API actors (lower priority)
//			const usersActors = context.actorsBySelector[usersSelector]; // Users actors (higher priority)
//
//			const apiRoute = context.routes?.[0];
//			const usersRoute = context.routes?.[1];
//
//			console.log(
//				"API selector:",
//				apiSelector,
//				"with actors:",
//				apiActors,
//			);
//			console.log(
//				"Users selector:",
//				usersSelector,
//				"with actors:",
//				usersActors,
//			);
//
//			// 1. First verify we can access both sets of actors directly with their exact paths
//			console.log(
//				"\nStep 1: Verifying direct access to both actor groups",
//			);
//
//			// 1.1 Test the less specific path (/api) - should route to first set of actors
//			console.log("Testing /api path - should route to API actors");
//			const apiResult = await testRoute(
//				context.hostname,
//				"/api",
//				2,
//				10,
//				apiRoute,
//			);
//			if (apiResult.size < 2) {
//				throw new Error(
//					"Path priority test failed: Could not reach actors at /api",
//				);
//			}
//
//			// Check we got responses from the API actors
//			let matchedApiActors = 0;
//			for (const id of apiResult) {
//				if (apiActors.includes(id)) {
//					matchedApiActors++;
//				}
//			}
//
//			if (matchedApiActors === apiResult.size) {
//				console.log(
//					"✅ All requests to /api routed to API actors as expected",
//				);
//			} else {
//				console.log(
//					`❌ Expected all requests to /api to route to API actors, but only ${matchedApiActors}/${apiResult.size} did`,
//				);
//				throw new Error(
//					`Path priority test failed: ${matchedApiActors}/${apiResult.size} requests to /api routed to API actors`,
//				);
//			}
//
//			// 1.2 Test the more specific path (/api/users) - should route to users actors
//			console.log(
//				"\nTesting /api/users path - should route to Users actors",
//			);
//			const usersResult = await testRoute(
//				context.hostname,
//				"/api/users",
//				2,
//				10,
//				usersRoute,
//			);
//			if (usersResult.size < 2) {
//				throw new Error(
//					"Path priority test failed: Could not reach actors at /api/users",
//				);
//			}
//
//			// Check we got responses from the Users actors
//			let matchedUsersActors = 0;
//			for (const id of usersResult) {
//				if (usersActors.includes(id)) {
//					matchedUsersActors++;
//				}
//			}
//
//			if (matchedUsersActors === usersResult.size) {
//				console.log(
//					"✅ All requests to /api/users routed to Users actors as expected",
//				);
//			} else {
//				console.log(
//					`❌ Expected all requests to /api/users to route to Users actors, but only ${matchedUsersActors}/${usersResult.size} did`,
//				);
//				throw new Error(
//					`Path priority test failed: ${matchedUsersActors}/${usersResult.size} requests to /api/users routed to Users actors`,
//				);
//			}
//
//			// 2. Test a path that would match the /api prefix but is a subpath of /api/users
//			// Should go to the /api/users actors as the more specific path has higher priority
//			console.log("\nStep 2: Testing a subpath priority");
//			console.log(
//				"Testing /api/users/123 path - should route to Users actors (more specific path)",
//			);
//			const subpathResult = await testRoute(
//				context.hostname,
//				"/api/users/123",
//				2,
//				10,
//				usersRoute,
//			);
//			if (subpathResult.size < 2) {
//				throw new Error(
//					"Path priority test failed: Could not reach actors at /api/users/123",
//				);
//			}
//
//			// Check if we got responses from the expected actors (should be from Users actors path)
//			let matchedSubpathUsers = 0;
//			for (const id of subpathResult) {
//				if (usersActors.includes(id)) {
//					matchedSubpathUsers++;
//				}
//			}
//
//			if (matchedSubpathUsers === subpathResult.size) {
//				console.log(
//					"✅ All requests for /api/users/123 routed to Users actors (more specific path) as expected",
//				);
//			} else {
//				console.log(
//					`❌ Expected all requests to route to Users actors (more specific path), but only ${matchedSubpathUsers}/${subpathResult.size} did`,
//				);
//				throw new Error(
//					`Path priority test failed for subpath: ${matchedSubpathUsers}/${subpathResult.size} requests routed to the Users actors`,
//				);
//			}
//
//			// 3. Test a path that matches the /api prefix but is NOT a subpath of /api/users
//			// Should go to the /api actors
//			console.log("\nStep 3: Testing another /api subpath");
//			console.log(
//				"Testing /api/other path - should route to API actors (because it's not under /api/users)",
//			);
//			const otherResult = await testRoute(
//				context.hostname,
//				"/api/other",
//				2,
//				10,
//				apiRoute,
//			);
//			if (otherResult.size < 2) {
//				throw new Error(
//					"Path priority test failed: Could not reach actors at /api/other",
//				);
//			}
//
//			// Check if we got responses from the API actors
//			let matchedOtherApi = 0;
//			for (const id of otherResult) {
//				if (apiActors.includes(id)) {
//					matchedOtherApi++;
//				}
//			}
//
//			if (matchedOtherApi === otherResult.size) {
//				console.log(
//					"✅ All requests for /api/other routed to API actors as expected",
//				);
//			} else {
//				console.log(
//					`❌ Expected all requests to route to API actors, but only ${matchedOtherApi}/${otherResult.size} did`,
//				);
//				throw new Error(
//					`Path priority test failed for other subpath: ${matchedOtherApi}/${otherResult.size} requests routed to the API actors`,
//				);
//			}
//		},
//	);
//}
//
//async function testEmptyPath(): Promise<boolean> {
//	return await setupTest(
//		{
//			name: "Empty Path Test",
//			numSelectors: 1,
//			routes: [
//				{
//					path: "",
//					routeSubpaths: true,
//					stripPrefix: true,
//					selectorIndex: 0,
//				},
//			],
//		},
//		async (context) => {
//			// Get the actor IDs for the first selector
//			const selectorActors =
//				context.actorsBySelector[context.selectors[0]];
//			const route = context.routes?.[0];
//
//			// Test various paths that should all match due to empty path with routeSubpaths=true
//			console.log("Testing empty path which should match any path");
//
//			let result = await testRoute(context.hostname, "/", 2, 20, route);
//			if (result.size < 2) {
//				throw new Error(
//					"Empty path test failed: Could not reach actors at /",
//				);
//			}
//
//			result = await testRoute(context.hostname, "/api", 2, 20, route);
//			if (result.size < 2) {
//				throw new Error(
//					"Empty path test failed: Could not reach actors at /api",
//				);
//			}
//
//			result = await testRoute(context.hostname, "/users", 2, 20, route);
//			if (result.size < 2) {
//				throw new Error(
//					"Empty path test failed: Could not reach actors at /users",
//				);
//			}
//
//			result = await testRoute(
//				context.hostname,
//				"/deep/nested/path",
//				2,
//				20,
//				route,
//			);
//			if (result.size < 2) {
//				throw new Error(
//					"Empty path test failed: Could not reach actors at /deep/nested/path",
//				);
//			}
//		},
//	);
//}
//
//async function testNoStripPrefix(): Promise<boolean> {
//	return await setupTest(
//		{
//			name: "No Strip Prefix Test",
//			numSelectors: 1,
//			routes: [
//				{
//					path: "/prefix",
//					routeSubpaths: true,
//					stripPrefix: false,
//					selectorIndex: 0,
//				},
//			],
//		},
//		async (context) => {
//			// Get the actor IDs for the first selector
//			const selectorActors =
//				context.actorsBySelector[context.selectors[0]];
//			const route = context.routes?.[0];
//
//			// Test the exact path - the path in response should be the full path
//			console.log(
//				"Testing exact path with stripPrefix=false. Path should NOT be stripped.",
//			);
//			const result = await testRoute(
//				context.hostname,
//				"/prefix",
//				2,
//				20,
//				route,
//			);
//			if (result.size < 2) {
//				throw new Error(
//					"No strip prefix test failed: Could not reach actors at /prefix",
//				);
//			}
//
//			// Test a subpath - the path in response should be the full path again
//			console.log(
//				"Testing subpath with stripPrefix=false. Path should NOT be stripped.",
//			);
//			const subpathResult = await testRoute(
//				context.hostname,
//				"/prefix/subpath",
//				2,
//				20,
//				route,
//			);
//			if (subpathResult.size < 2) {
//				throw new Error(
//					"No strip prefix test failed: Could not reach actors at /prefix/subpath",
//				);
//			}
//		},
//	);
//}
//
//async function testMultipleRoutes(): Promise<boolean> {
//	return await setupTest(
//		{
//			name: "Multiple Routes Test",
//			numSelectors: 2,
//			routes: [
//				{
//					path: "/route1",
//					routeSubpaths: false,
//					stripPrefix: true,
//					selectorIndex: 0,
//				},
//				{
//					path: "/route2",
//					routeSubpaths: false,
//					stripPrefix: true,
//					selectorIndex: 1,
//				},
//			],
//		},
//		async (context) => {
//			// Get the actor IDs for both selectors
//			const selector1Actors =
//				context.actorsBySelector[context.selectors[0]];
//			const selector2Actors =
//				context.actorsBySelector[context.selectors[1]];
//
//			const route1 = context.routes?.[0];
//			const route2 = context.routes?.[1];
//
//			// Test first route
//			console.log("Testing first route /route1");
//			const result1 = await testRoute(
//				context.hostname,
//				"/route1",
//				2,
//				20,
//				route1,
//			);
//			if (result1.size < 2) {
//				throw new Error(
//					"Multiple routes test failed: Could not reach actors at /route1",
//				);
//			}
//
//			// Verify we got responses from the correct actors
//			let matchedActors1 = 0;
//			for (const id of result1) {
//				if (selector1Actors.includes(id)) {
//					matchedActors1++;
//				}
//			}
//
//			if (matchedActors1 === result1.size) {
//				console.log(
//					"✅ All requests to /route1 routed to route1 actors as expected",
//				);
//			} else {
//				console.log(
//					`❌ Expected all requests to route to route1 actors, but only ${matchedActors1}/${result1.size} did`,
//				);
//				throw new Error(
//					`Multiple routes test failed: ${matchedActors1}/${result1.size} requests to /route1 routed to route1 actors`,
//				);
//			}
//
//			// Test second route
//			console.log("Testing second route /route2");
//			const result2 = await testRoute(
//				context.hostname,
//				"/route2",
//				2,
//				20,
//				route2,
//			);
//			if (result2.size < 2) {
//				throw new Error(
//					"Multiple routes test failed: Could not reach actors at /route2",
//				);
//			}
//
//			// Verify we got responses from the correct actors
//			let matchedActors2 = 0;
//			for (const id of result2) {
//				if (selector2Actors.includes(id)) {
//					matchedActors2++;
//				}
//			}
//
//			if (matchedActors2 === result2.size) {
//				console.log(
//					"✅ All requests to /route2 routed to route2 actors as expected",
//				);
//			} else {
//				console.log(
//					`❌ Expected all requests to route to route2 actors, but only ${matchedActors2}/${result2.size} did`,
//				);
//				throw new Error(
//					`Multiple routes test failed: ${matchedActors2}/${result2.size} requests to /route2 routed to route2 actors`,
//				);
//			}
//		},
//	);
//}
//
//async function testQueryParameters(): Promise<boolean> {
//	return await setupTest(
//		{
//			name: "Query Parameters Test",
//			numSelectors: 1,
//			routes: [
//				{
//					path: "/query",
//					routeSubpaths: false,
//					stripPrefix: true,
//					selectorIndex: 0,
//				},
//			],
//		},
//		async (context) => {
//			// Get the actor IDs for the selector
//			const selectorActors =
//				context.actorsBySelector[context.selectors[0]];
//			const route = context.routes?.[0];
//
//			// Custom function to validate query parameters
//			async function testWithQueryValidation(
//				path: string,
//				expectedQuery: Record<string, string>,
//			): Promise<void> {
//				// Make the request
//				const testUrl = `http://localhost:7080${path}`;
//				console.log(
//					`Testing route at: ${testUrl} (with Host: ${context.hostname})`,
//				);
//
//				const data = await makeRequest(testUrl, context.hostname);
//				if (!data) {
//					throw new Error(`Failed to get response from ${path}`);
//				}
//
//				// Validate query parameters
//				console.log("Expected query parameters:", expectedQuery);
//				console.log("Actual query parameters:", data.query);
//
//				// Check that all expected parameters are present
//				let queryValidationPassed = true;
//				for (const [key, value] of Object.entries(expectedQuery)) {
//					if (data.query[key] !== value) {
//						console.error(
//							`❌ Query parameter validation failed for ${key}: Expected ${value}, got ${data.query[key]}`,
//						);
//						queryValidationPassed = false;
//					}
//				}
//
//				if (queryValidationPassed) {
//					console.log("✅ Query parameter validation passed");
//				} else {
//					throw new Error("Query parameter validation failed");
//				}
//			}
//
//			// Test path with simple query parameters
//			console.log("Testing path with simple query parameters");
//			const result = await testRoute(
//				context.hostname,
//				"/query?param=value&another=123",
//				2,
//				20,
//				route,
//			);
//			if (result.size < 2) {
//				throw new Error(
//					"Query parameters test failed: Could not reach actors at /query with query parameters",
//				);
//			}
//
//			// Validate simple query parameters with direct check
//			await testWithQueryValidation("/query?param=value&another=123", {
//				param: "value",
//				another: "123",
//			});
//
//			// Test more complex query parameters
//			console.log("Testing path with complex query parameters");
//			const complexResult = await testRoute(
//				context.hostname,
//				"/query?complex=test%20with%20spaces&array[]=1&array[]=2",
//				2,
//				20,
//				route,
//			);
//			if (complexResult.size < 2) {
//				throw new Error(
//					"Query parameters test failed: Could not reach actors at /query with complex query parameters",
//				);
//			}
//
//			// Validate complex query parameters with direct check
//			await testWithQueryValidation(
//				"/query?complex=test%20with%20spaces",
//				{
//					complex: "test with spaces",
//				},
//			);
//		},
//	);
//}
//
//async function testSpecialCharacters(): Promise<boolean> {
//	return await setupTest(
//		{
//			name: "Special Characters Test",
//			numSelectors: 1,
//			routes: [
//				{
//					path: "/special-chars",
//					routeSubpaths: true,
//					stripPrefix: true,
//					selectorIndex: 0,
//				},
//			],
//		},
//		async (context) => {
//			// Get the actor IDs for the selector
//			const selectorActors =
//				context.actorsBySelector[context.selectors[0]];
//			const route = context.routes?.[0];
//
//			// Test paths with special characters
//			console.log("Testing path with hyphens");
//			let result = await testRoute(
//				context.hostname,
//				"/special-chars/with-hyphens",
//				2,
//				20,
//				route,
//			);
//			if (result.size < 2) {
//				throw new Error(
//					"Special characters test failed: Could not reach actors at path with hyphens",
//				);
//			}
//
//			console.log("Testing path with underscores");
//			result = await testRoute(
//				context.hostname,
//				"/special-chars/with_underscores",
//				2,
//				20,
//				route,
//			);
//			if (result.size < 2) {
//				throw new Error(
//					"Special characters test failed: Could not reach actors at path with underscores",
//				);
//			}
//
//			console.log("Testing path with dots");
//			result = await testRoute(
//				context.hostname,
//				"/special-chars/with.dots",
//				2,
//				20,
//				route,
//			);
//			if (result.size < 2) {
//				throw new Error(
//					"Special characters test failed: Could not reach actors at path with dots",
//				);
//			}
//
//			console.log("Testing path with encoded characters");
//			result = await testRoute(
//				context.hostname,
//				"/special-chars/encoded%20space",
//				2,
//				20,
//				route,
//			);
//			if (result.size < 2) {
//				throw new Error(
//					"Special characters test failed: Could not reach actors at path with encoded characters",
//				);
//			}
//		},
//	);
//}
//
//async function testLargeActorPool(): Promise<boolean> {
//	return await setupTest(
//		{
//			name: "Large Actor Pool Test",
//			numActors: 10, // Use 10 actors instead of the default 2
//			numSelectors: 1,
//			routes: [
//				{
//					path: "/large-pool",
//					routeSubpaths: false,
//					stripPrefix: true,
//					selectorIndex: 0,
//				},
//			],
//		},
//		async (context) => {
//			// Get the actor IDs for the selector
//			const selectorActors =
//				context.actorsBySelector[context.selectors[0]];
//			const route = context.routes?.[0];
//
//			// Test with larger number of actors to verify load balancing
//			console.log("Testing with larger actor pool (10 actors)");
//
//			// Make 20 requests and track unique actor IDs seen
//			const allActorsResult = new Set<string>();
//
//			// Make multiple requests to see distribution
//			for (let i = 0; i < 3; i++) {
//				console.log(`Batch ${i + 1} of requests to large actor pool:`);
//				const result = await testRoute(
//					context.hostname,
//					"/large-pool",
//					5,
//					20,
//					route,
//				);
//
//				// Add these results to our overall set
//				for (const id of result) {
//					allActorsResult.add(id);
//				}
//
//				// Wait a bit between batches
//				await new Promise((resolve) => setTimeout(resolve, 500));
//			}
//
//			// Verify we've seen a good number of unique actors (at least 7 out of 10)
//			if (allActorsResult.size < 7) {
//				console.log(
//					`❌ Only reached ${allActorsResult.size}/10 unique actors, expected at least 7`,
//				);
//				throw new Error(
//					`Large actor pool test: Only reached ${allActorsResult.size}/10 unique actors`,
//				);
//			} else {
//				console.log(
//					`✅ Reached ${allActorsResult.size}/10 unique actors across all requests`,
//				);
//			}
//		},
//	);
//}
//
//async function testLongPath(): Promise<boolean> {
//	return await setupTest(
//		{
//			name: "Long Path Test",
//			numSelectors: 1,
//			routes: [
//				{
//					path: "/long",
//					routeSubpaths: true,
//					stripPrefix: true,
//					selectorIndex: 0,
//				},
//			],
//		},
//		async (context) => {
//			// Get the actor IDs for the selector
//			const selectorActors =
//				context.actorsBySelector[context.selectors[0]];
//			const route = context.routes?.[0];
//
//			// Create a very long path
//			let longPathSegment = "";
//			for (let i = 0; i < 10; i++) {
//				longPathSegment += "segment-" + i + "/";
//			}
//			const longPath = `/long/${longPathSegment}end`;
//
//			console.log("Testing very long path");
//			console.log(`Path length: ${longPath.length} characters`);
//
//			const result = await testRoute(
//				context.hostname,
//				longPath,
//				2,
//				20,
//				route,
//			);
//			if (result.size < 2) {
//				throw new Error(
//					"Long path test failed: Could not reach actors with very long path",
//				);
//			}
//		},
//	);
//}
//
//async function testMixedPrefixStripping(): Promise<boolean> {
//	return await setupTest(
//		{
//			name: "Mixed Prefix Stripping Test",
//			numSelectors: 2,
//			routes: [
//				{
//					path: "/strip",
//					routeSubpaths: true,
//					stripPrefix: true,
//					selectorIndex: 0,
//				},
//				{
//					path: "/nostrip",
//					routeSubpaths: true,
//					stripPrefix: false,
//					selectorIndex: 1,
//				},
//			],
//		},
//		async (context) => {
//			// Get the actor IDs for both selectors
//			const stripActors = context.actorsBySelector[context.selectors[0]];
//			const noStripActors =
//				context.actorsBySelector[context.selectors[1]];
//
//			const stripRoute = context.routes?.[0];
//			const noStripRoute = context.routes?.[1];
//
//			// Test the strip prefix route
//			console.log("Testing route with stripPrefix=true");
//			const stripResult = await testRoute(
//				context.hostname,
//				"/strip/subpath",
//				2,
//				20,
//				stripRoute,
//			);
//			if (stripResult.size < 2) {
//				throw new Error(
//					"Mixed prefix stripping test failed: Could not reach actors at /strip/subpath",
//				);
//			}
//
//			// Test the no strip prefix route
//			console.log("Testing route with stripPrefix=false");
//			const noStripResult = await testRoute(
//				context.hostname,
//				"/nostrip/subpath",
//				2,
//				20,
//				noStripRoute,
//			);
//			if (noStripResult.size < 2) {
//				throw new Error(
//					"Mixed prefix stripping test failed: Could not reach actors at /nostrip/subpath",
//				);
//			}
//
//			// Verify we got responses from the correct actors for strip route
//			let matchedStripActors = 0;
//			for (const id of stripResult) {
//				if (stripActors.includes(id)) {
//					matchedStripActors++;
//				}
//			}
//
//			if (matchedStripActors === stripResult.size) {
//				console.log(
//					"✅ All requests to /strip/subpath routed to strip actors as expected",
//				);
//			} else {
//				console.log(
//					`❌ Expected all requests to route to strip actors, but only ${matchedStripActors}/${stripResult.size} did`,
//				);
//				throw new Error(
//					`Mixed prefix stripping test failed: ${matchedStripActors}/${stripResult.size} requests routed to strip actors`,
//				);
//			}
//
//			// Verify we got responses from the correct actors for no-strip route
//			let matchedNoStripActors = 0;
//			for (const id of noStripResult) {
//				if (noStripActors.includes(id)) {
//					matchedNoStripActors++;
//				}
//			}
//
//			if (matchedNoStripActors === noStripResult.size) {
//				console.log(
//					"✅ All requests to /nostrip/subpath routed to no-strip actors as expected",
//				);
//			} else {
//				console.log(
//					`❌ Expected all requests to route to no-strip actors, but only ${matchedNoStripActors}/${noStripResult.size} did`,
//				);
//				throw new Error(
//					`Mixed prefix stripping test failed: ${matchedNoStripActors}/${noStripResult.size} requests routed to no-strip actors`,
//				);
//			}
//		},
//	);
//}
//
//async function test404Response(): Promise<boolean> {
//	return await setupTest(
//		{
//			name: "404 Response Test",
//			numSelectors: 1,
//			routes: [
//				{
//					path: "/test-path",
//					routeSubpaths: false,
//					stripPrefix: true,
//					selectorIndex: 0,
//				},
//			],
//		},
//		async (context) => {
//			// Get the actor IDs for the selector
//			const selectorActors =
//				context.actorsBySelector[context.selectors[0]];
//			const route = context.routes?.[0];
//
//			// First verify our route works
//			console.log("Verifying route /test-path exists and works");
//			const result = await testRoute(
//				context.hostname,
//				"/test-path",
//				2,
//				20,
//				route,
//			);
//			if (result.size < 2) {
//				throw new Error(
//					"404 test setup failed: Could not reach actors at /test-path",
//				);
//			}
//
//			// Now test a path that doesn't match any route - should return 404
//			console.log("Testing path that should 404: /non-existent-path");
//
//			try {
//				// We expect this to return an empty set (no matches)
//				const notFoundResult = await testRoute(
//					context.hostname,
//					"/non-existent-path",
//					2,
//					5,
//				);
//
//				// If we get here with results, it means the 404 test failed
//				if (notFoundResult.size > 0) {
//					console.error(
//						"❌ Expected 404 for /non-existent-path but got a successful response",
//					);
//					throw new Error(
//						"404 test failed: Got a successful response instead of 404",
//					);
//				} else {
//					console.log(
//						"✅ Correctly received no matches for non-existent path",
//					);
//				}
//			} catch (error) {
//				// Check if this is our expected path validation error
//				if (
//					error instanceof Error &&
//					error.message.includes("Path validation failed")
//				) {
//					console.error(
//						"❌ Unexpected path returned for non-existent route",
//					);
//					throw error;
//				}
//
//				// If it's a different error (like connection refused), that's expected for a 404
//				console.log(
//					"✅ Received expected error for non-existent path:",
//					error.message,
//				);
//			}
//
//			// Test with a different hostname that doesn't have any routes
//			const randomHostname = `nonexistent-${crypto.randomBytes(4).toString("hex")}.rivet-job.local`;
//			console.log("Testing with non-existent hostname:", randomHostname);
//
//			try {
//				const nonExistentHostResult = await makeRequest(
//					`http://localhost:7080/test`,
//					randomHostname,
//				);
//
//				// If we get a response, it's unexpected
//				if (nonExistentHostResult) {
//					console.error(
//						"❌ Expected 404 for non-existent hostname but got a response:",
//						nonExistentHostResult,
//					);
//					throw new Error(
//						"404 test failed: Got a response for non-existent hostname",
//					);
//				} else {
//					console.log(
//						"✅ Correctly received no response for non-existent hostname",
//					);
//				}
//			} catch (error) {
//				// This is expected - we should get an error
//				console.log(
//					"✅ Received expected error for non-existent hostname:",
//					error.message,
//				);
//			}
//		},
//	);
//}
//
//async function run() {
//	try {
//		const tests = [
//			{ name: "Basic Route Test", fn: testBasicRoute },
//			{ name: "Path Prefix Test", fn: testPathPrefix },
//			{ name: "Exact Path Test", fn: testExactPath },
//			{ name: "Path Priority Test", fn: testPathPriority },
//			{ name: "Empty Path Test", fn: testEmptyPath },
//			{ name: "No Strip Prefix Test", fn: testNoStripPrefix },
//			{ name: "Multiple Routes Test", fn: testMultipleRoutes },
//			{ name: "Query Parameters Test", fn: testQueryParameters },
//			{ name: "Special Characters Test", fn: testSpecialCharacters },
//			{ name: "Large Actor Pool Test", fn: testLargeActorPool },
//			{ name: "Long Path Test", fn: testLongPath },
//			{
//				name: "Mixed Prefix Stripping Test",
//				fn: testMixedPrefixStripping,
//			},
//			{ name: "404 Response Test", fn: test404Response }, // 404 test should run last
//		];
//
//		for (const test of tests) {
//			console.log(`\nRunning test: ${test.name}`);
//			const testPassed = await test.fn();
//
//			// If any test fails, exit immediately
//			if (!testPassed) {
//				console.error(
//					`\n❌ Test "${test.name}" failed. Exiting test suite.`,
//				);
//				process.exit(1);
//			}
//		}
//
//		console.log("\n=== All tests completed successfully ===\n");
//		process.exit(0);
//	} catch (error) {
//		console.error("Error running tests:", error);
//		process.exit(1);
//	}
//}
//
//// Run the test
//run().catch((error) => {
//	console.error("Unhandled error in test suite:", error);
//	process.exit(1);
//});
