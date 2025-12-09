// import fs from "node:fs/promises";
// import path from "node:path";
//
// interface RailwayConfig {
// 	apiToken: string;
// 	workspaceId: string;
// 	projectId: string;
// }
//
// interface GraphQLResponse<T> {
// 	data?: T;
// 	errors?: Array<{ message: string }>;
// }
//
// interface ProjectData {
// 	id: string;
// 	name: string;
// 	environments: {
// 		edges: Array<{
// 			node: {
// 				id: string;
// 				name: string;
// 			};
// 		}>;
// 	};
// 	services: {
// 		edges: Array<{
// 			node: {
// 				id: string;
// 				name: string;
// 			};
// 		}>;
// 	};
// }
//
// interface EnvironmentData {
// 	id: string;
// 	name: string;
// 	projectId: string;
// }
//
// interface ServiceData {
// 	id: string;
// 	name: string;
// 	projectId: string;
// }
//
// interface TemplateData {
// 	id: string;
// 	code: string;
// 	status: string;
// }
//
// function sanitizeEnvironmentName(name: string): string {
// 	// Railway environment names must be:
// 	// - Max 32 characters
// 	// - Alphanumeric and hyphens only
// 	// - No consecutive hyphens
// 	// - Start with a letter or number
// 	let sanitized = name
// 		.toLowerCase()
// 		.replace(/[^a-z0-9-]/g, "-") // Replace invalid chars with hyphens
// 		.replace(/-+/g, "-") // Replace consecutive hyphens with single hyphen
// 		.replace(/^-|-$/g, ""); // Remove leading/trailing hyphens
//
// 	// Truncate to 32 characters
// 	if (sanitized.length > 32) {
// 		sanitized = sanitized.substring(0, 32).replace(/-$/, "");
// 	}
//
// 	return sanitized;
// }
//
// async function graphqlRequest<T>(
// 	query: string,
// 	variables: Record<string, any>,
// 	config: RailwayConfig,
// ): Promise<T> {
// 	const response = await fetch("https://backboard.railway.com/graphql/v2", {
// 		method: "POST",
// 		headers: {
// 			"Content-Type": "application/json",
// 			Authorization: `Bearer ${config.apiToken}`,
// 		},
// 		body: JSON.stringify({ query, variables }),
// 	});
//
// 	if (!response.ok) {
// 		throw new Error(
// 			`Railway API request failed: ${response.status} ${response.statusText}`,
// 		);
// 	}
//
// 	const result: GraphQLResponse<T> = await response.json();
//
// 	if (result.errors) {
// 		throw new Error(
// 			`Railway GraphQL errors: ${result.errors.map((e) => e.message).join(", ")}`,
// 		);
// 	}
//
// 	if (!result.data) {
// 		throw new Error("Railway API returned no data");
// 	}
//
// 	return result.data;
// }
//
// async function getOrCreateProject(
// 	config: RailwayConfig,
// ): Promise<ProjectData> {
// 	// First try to get the existing project
// 	const getProjectQuery = `
// 		query GetProject($projectId: String!) {
// 			project(id: $projectId) {
// 				id
// 				name
// 				environments {
// 					edges {
// 						node {
// 							id
// 							name
// 						}
// 					}
// 				}
// 				services {
// 					edges {
// 						node {
// 							id
// 							name
// 						}
// 					}
// 				}
// 			}
// 		}
// 	`;
//
// 	try {
// 		const data = await graphqlRequest<{ project: ProjectData }>(
// 			getProjectQuery,
// 			{ projectId: config.projectId },
// 			config,
// 		);
// 		return data.project;
// 	} catch (error) {
// 		throw new Error(
// 			`Failed to get Railway project ${config.projectId}: ${error}`,
// 		);
// 	}
// }
//
// async function getOrCreateEnvironment(
// 	projectId: string,
// 	environmentName: string,
// 	config: RailwayConfig,
// ): Promise<EnvironmentData> {
// 	// Get project to check existing environments
// 	const project = await getOrCreateProject(config);
//
// 	// Check if environment already exists
// 	const existingEnv = project.environments.edges.find(
// 		(edge) => edge.node.name === environmentName,
// 	);
//
// 	if (existingEnv) {
// 		console.log(`  ✓ Found existing environment: ${environmentName}`);
// 		return existingEnv.node;
// 	}
//
// 	// Create new environment
// 	console.log(`  + Creating environment: ${environmentName}`);
// 	const createEnvMutation = `
// 		mutation CreateEnvironment($input: EnvironmentCreateInput!) {
// 			environmentCreate(input: $input) {
// 				id
// 				name
// 				projectId
// 			}
// 		}
// 	`;
//
// 	const data = await graphqlRequest<{ environmentCreate: EnvironmentData }>(
// 		createEnvMutation,
// 		{
// 			input: {
// 				projectId,
// 				name: environmentName,
// 				skipInitialDeploys: true,
// 			},
// 		},
// 		config,
// 	);
//
// 	return data.environmentCreate;
// }
//
// async function deleteService(
// 	serviceId: string,
// 	environmentId: string,
// 	config: RailwayConfig,
// ): Promise<void> {
// 	const deleteServiceMutation = `
// 		mutation DeleteService($id: String!, $environmentId: String) {
// 			serviceDelete(id: $id, environmentId: $environmentId)
// 		}
// 	`;
//
// 	await graphqlRequest(
// 		deleteServiceMutation,
// 		{
// 			id: serviceId,
// 			environmentId,
// 		},
// 		config,
// 	);
// }
//
// async function clearServicesInEnvironment(
// 	projectId: string,
// 	environmentId: string,
// 	config: RailwayConfig,
// ): Promise<void> {
// 	const project = await getOrCreateProject(config);
//
// 	// Delete all services in this environment
// 	for (const serviceEdge of project.services.edges) {
// 		console.log(`  🗑️  Deleting existing service: ${serviceEdge.node.name}`);
// 		await deleteService(serviceEdge.node.id, environmentId, config);
// 	}
// }
//
// async function createAndConnectService(
// 	projectId: string,
// 	environmentId: string,
// 	serviceName: string,
// 	repoFullName: string,
// 	branch: string,
// 	rootDirectory: string,
// 	config: RailwayConfig,
// ): Promise<ServiceData> {
// 	// Create new service in the specific environment
// 	console.log(`  + Creating service: ${serviceName}`);
// 	const createServiceMutation = `
// 		mutation CreateService($input: ServiceCreateInput!) {
// 			serviceCreate(input: $input) {
// 				id
// 				name
// 				projectId
// 			}
// 		}
// 	`;
//
// 	const data = await graphqlRequest<{ serviceCreate: ServiceData }>(
// 		createServiceMutation,
// 		{
// 			input: {
// 				projectId,
// 				environmentId,
// 				name: serviceName,
// 			},
// 		},
// 		config,
// 	);
//
// 	const service = data.serviceCreate;
//
// 	// Connect the service to the repo
// 	console.log(`  → Connecting service to repo: ${repoFullName}`);
// 	const connectServiceMutation = `
// 		mutation ConnectService($id: String!, $input: ServiceConnectInput!) {
// 			serviceConnect(id: $id, input: $input)
// 		}
// 	`;
//
// 	await graphqlRequest(
// 		connectServiceMutation,
// 		{
// 			id: service.id,
// 			input: {
// 				repo: repoFullName,
// 				branch,
// 			},
// 		},
// 		config,
// 	);
//
// 	// Update service instance with root directory immediately
// 	console.log(`  → Setting root directory: ${rootDirectory}`);
// 	const updateServiceMutation = `
// 		mutation UpdateServiceInstance(
// 			$serviceId: String!
// 			$environmentId: String
// 			$input: ServiceInstanceUpdateInput!
// 		) {
// 			serviceInstanceUpdate(
// 				serviceId: $serviceId
// 				environmentId: $environmentId
// 				input: $input
// 			)
// 		}
// 	`;
//
// 	await graphqlRequest(
// 		updateServiceMutation,
// 		{
// 			serviceId: service.id,
// 			environmentId,
// 			input: {
// 				rootDirectory,
// 			},
// 		},
// 		config,
// 	);
//
// 	return service;
// }
//
// async function updateServiceInstance(
// 	serviceId: string,
// 	environmentId: string,
// 	rootDirectory: string,
// 	startCommand: string | undefined,
// 	buildCommand: string | undefined,
// 	config: RailwayConfig,
// ): Promise<void> {
// 	console.log(`  → Updating service instance configuration`);
// 	const updateServiceMutation = `
// 		mutation UpdateServiceInstance(
// 			$serviceId: String!
// 			$environmentId: String
// 			$input: ServiceInstanceUpdateInput!
// 		) {
// 			serviceInstanceUpdate(
// 				serviceId: $serviceId
// 				environmentId: $environmentId
// 				input: $input
// 			)
// 		}
// 	`;
//
// 	const input: Record<string, any> = {
// 		rootDirectory,
// 	};
//
// 	if (startCommand) {
// 		input.startCommand = startCommand;
// 	}
//
// 	if (buildCommand) {
// 		input.buildCommand = buildCommand;
// 	}
//
// 	await graphqlRequest(
// 		updateServiceMutation,
// 		{
// 			serviceId,
// 			environmentId,
// 			input,
// 		},
// 		config,
// 	);
// }
//
// async function generateTemplate(
// 	projectId: string,
// 	environmentId: string,
// 	config: RailwayConfig,
// ): Promise<TemplateData> {
// 	console.log(`  → Generating template`);
// 	const generateTemplateMutation = `
// 		mutation GenerateTemplate($input: TemplateGenerateInput!) {
// 			templateGenerate(input: $input) {
// 				id
// 				code
// 				status
// 			}
// 		}
// 	`;
//
// 	const data = await graphqlRequest<{ templateGenerate: TemplateData }>(
// 		generateTemplateMutation,
// 		{
// 			input: {
// 				projectId,
// 				environmentId,
// 			},
// 		},
// 		config,
// 	);
//
// 	return data.templateGenerate;
// }
//
// async function publishTemplate(
// 	templateId: string,
// 	category: string,
// 	description: string,
// 	readme: string,
// 	config: RailwayConfig,
// ): Promise<TemplateData> {
// 	console.log(`  → Publishing template`);
// 	const publishTemplateMutation = `
// 		mutation PublishTemplate($id: String!, $input: TemplatePublishInput!) {
// 			templatePublish(id: $id, input: $input) {
// 				id
// 				code
// 				status
// 			}
// 		}
// 	`;
//
// 	const data = await graphqlRequest<{ templatePublish: TemplateData }>(
// 		publishTemplateMutation,
// 		{
// 			id: templateId,
// 			input: {
// 				category,
// 				description,
// 				readme,
// 			},
// 		},
// 		config,
// 	);
//
// 	return data.templatePublish;
// }
//
// export interface ExampleData {
// 	name: string;
// 	displayName: string;
// 	description: string;
// 	readmePath: string;
// 	startCommand?: string;
// 	buildCommand?: string;
// }
//
// export async function syncRailwayTemplate(
// 	example: ExampleData,
// 	config: RailwayConfig,
// ): Promise<void> {
// 	console.log(`\n🚂 Syncing Railway template for: ${example.name}`);
//
// 	try {
// 		// 1. Get or create environment for this example
// 		const environmentName = sanitizeEnvironmentName(example.name);
// 		if (environmentName !== example.name) {
// 			console.log(`  ℹ️  Sanitized name: ${environmentName}`);
// 		}
// 		const environment = await getOrCreateEnvironment(
// 			config.projectId,
// 			environmentName,
// 			config,
// 		);
//
// 		// 2. Clear existing services in the environment
// 		await clearServicesInEnvironment(
// 			config.projectId,
// 			environment.id,
// 			config,
// 		);
//
// 		// 3. Create and connect service with proper configuration
// 		const serviceName = sanitizeEnvironmentName(example.name);
// 		const rootDirectory = `examples/${example.name}`;
// 		const service = await createAndConnectService(
// 			config.projectId,
// 			environment.id,
// 			serviceName,
// 			"rivet-dev/rivet",
// 			"main",
// 			rootDirectory,
// 			config,
// 		);
//
// 		// 4. Update additional service configuration (start/build commands)
// 		if (example.startCommand || example.buildCommand) {
// 			console.log(`  → Configuring start/build commands`);
// 			await updateServiceInstance(
// 				service.id,
// 				environment.id,
// 				rootDirectory,
// 				example.startCommand,
// 				example.buildCommand,
// 				config,
// 			);
// 		}
//
// 		// 5. Generate template from environment
// 		const template = await generateTemplate(
// 			config.projectId,
// 			environment.id,
// 			config,
// 		);
//
// 		// 6. Read README content for template
// 		const readmeContent = await fs.readFile(example.readmePath, "utf-8");
//
// 		// 7. Publish template
// 		await publishTemplate(
// 			template.id,
// 			"Starter",
// 			example.description,
// 			readmeContent,
// 			config,
// 		);
//
// 		console.log(`  ✅ Template synced successfully (code: ${template.code})`);
// 	} catch (error) {
// 		console.error(`  ❌ Failed to sync template: ${error}`);
// 		throw error;
// 	}
// }
//
// export async function loadRailwayConfig(): Promise<RailwayConfig | null> {
// 	// Check if Railway sync should be skipped
// 	if (process.env.SKIP_RAILWAY_TEMPLATES === "true") {
// 		console.log("\n⏭️  Skipping Railway template sync (SKIP_RAILWAY_TEMPLATES=true)");
// 		return null;
// 	}
//
// 	// Check for required environment variables
// 	const apiToken = process.env.RAILWAY_API_TOKEN;
// 	const workspaceId = process.env.RAILWAY_WORKSPACE_ID;
// 	const projectId = process.env.RAILWAY_PROJECT_ID;
//
// 	if (!apiToken || !workspaceId || !projectId) {
// 		const missing: string[] = [];
// 		if (!apiToken) missing.push("RAILWAY_API_TOKEN");
// 		if (!workspaceId) missing.push("RAILWAY_WORKSPACE_ID");
// 		if (!projectId) missing.push("RAILWAY_PROJECT_ID");
//
// 		throw new Error(
// 			`Missing required Railway environment variables: ${missing.join(", ")}\n` +
// 			`Set these variables or use SKIP_RAILWAY_TEMPLATES=true to skip Railway sync.`
// 		);
// 	}
//
// 	return {
// 		apiToken,
// 		workspaceId,
// 		projectId,
// 	};
// }
