import { RivetClient as CloudClient } from "@rivet-gg/cloud";
import { RivetClient } from "@rivetkit/engine-api-full";
import {
	configureRivetServerless,
	type DeployRequest,
	deployToFreestyle,
	generateNamespaceName,
	type LogCallback,
} from "./utils.ts";

export async function deployWithRivetCloud(
	req: DeployRequest,
	log: LogCallback,
) {
	if (!("cloud" in req.kind)) {
		throw new Error("Expected cloud deployment request");
	}

	const {
		cloudEndpoint: apiUrl,
		cloudToken: apiToken,
		engineEndpoint: endpoint,
	} = req.kind.cloud;
	const { freestyleDomain, freestyleApiKey } = req;

	const cloudRivet = new CloudClient({
		baseUrl: apiUrl,
		token: apiToken,
	});

	const { project, organization } = await cloudRivet.apiTokens.inspect();

	const namespaceName = generateNamespaceName();

	await log(`Creating namespace ${namespaceName}`);
	const { namespace } = await cloudRivet.namespaces.create(project, {
		name: namespaceName,
		displayName: namespaceName.substring(0, 16),
		org: organization,
	});

	const { token: runnerToken } =
		await cloudRivet.namespaces.createSecretToken(project, namespace.name, {
			name: `${namespaceName}-runner-token`,
			org: organization,
		});

	const { token: publishableToken } =
		await cloudRivet.namespaces.createPublishableToken(
			project,
			namespace.name,
			{
				org: organization,
			},
		);

	const { token: accessToken } =
		await cloudRivet.namespaces.createAccessToken(project, namespace.name, {
			org: organization,
		});

	const datacenter = req.datacenter || "us-west-1";

	// Deploy to Freestyle
	const { deploymentId } = await deployToFreestyle({
		registryCode: req.registryCode,
		appCode: req.appCode,
		domain: freestyleDomain,
		apiKey: freestyleApiKey,
		envVars: {
			VITE_RIVET_ENDPOINT: endpoint,
			VITE_RIVET_NAMESPACE: namespace.access.engineNamespaceName,
			VITE_RIVET_TOKEN: publishableToken,
			VITE_RIVET_DATACENTER: datacenter,
			RIVET_ENDPOINT: endpoint,
			RIVET_NAMESPACE: namespace.access.engineNamespaceName,
			RIVET_RUNNER_TOKEN: runnerToken,
			RIVET_PUBLISHABLE_TOKEN: publishableToken,
		},
		log,
	});

	// Update runner config
	const engineRivet = new RivetClient({
		environment: endpoint,
		token: accessToken,
	});

	await configureRivetServerless({
		rivet: engineRivet,
		domain: freestyleDomain,
		namespace: namespace.access.engineNamespaceName,
		datacenter,
		log,
	});

	return {
		success: true,
		dashboardUrl: `https://dashboard.rivet.dev/orgs/${organization}/projects/${project}/ns/${namespace.name}`,
		freestyleUrl: `https://admin.freestyle.sh/dashboard/deployments/${deploymentId}`,
		tokens: {
			runnerToken,
			publishableToken,
			accessToken,
		},
	};
}
