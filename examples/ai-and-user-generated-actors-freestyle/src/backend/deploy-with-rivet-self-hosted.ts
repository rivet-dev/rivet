import { RivetClient } from "@rivetkit/engine-api-full";
import {
	configureRivetServerless,
	type DeployRequest,
	deployToFreestyle,
	generateNamespaceName,
	type LogCallback,
} from "./utils";

export async function deployWithRivetSelfHosted(
	req: DeployRequest,
	log: LogCallback,
) {
	if (!("selfHosted" in req.kind)) {
		throw new Error("Expected self-hosted deployment request");
	}

	const { endpoint, token } = req.kind.selfHosted;
	const { freestyleDomain, freestyleApiKey } = req;

	const rivet = new RivetClient({
		environment: endpoint,
		token: token,
	});

	const namespaceName = generateNamespaceName();

	await log(`Creating namespace ${namespaceName}`);
	const { namespace } = await rivet.namespaces.create({
		displayName: namespaceName,
		name: namespaceName,
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
			VITE_RIVET_NAMESPACE: namespace.name,
			VITE_RIVET_TOKEN: token,
			VITE_RIVET_DATACENTER: datacenter,
			RIVET_ENDPOINT: endpoint,
			RIVET_NAMESPACE: namespace.name,
			RIVET_RUNNER_TOKEN: token,
			RIVET_PUBLISHABLE_TOKEN: token,
		},
		log,
	});

	await configureRivetServerless({
		rivet,
		domain: freestyleDomain,
		namespace: namespace.name,
		datacenter,
		log,
	});

	return {
		success: true,
		freestyleUrl: `https://admin.freestyle.sh/dashboard/deployments/${deploymentId}`,
	};
}
