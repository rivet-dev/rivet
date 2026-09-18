import {
	BYOC_QUICKSTART_DOCS_URL,
	BYOC_SETUP_KIT_URL,
} from "../../content/byoc";
import {
	CLUSTER_CONFIG_FILENAME,
	serializeClusterConfig,
} from "./cluster-config";

/** Sensitive clipboard text, never a downloadable or uploaded release asset. */
export function serializeAgentInstructions(
	clusterId: string | undefined,
	apiUrl: string,
	operatorToken: string | undefined,
): string | null {
	const config = serializeClusterConfig(clusterId, apiUrl);
	if (
		!config ||
		!operatorToken ||
		!/^[\x21-\x7e]{32,16384}$/.test(operatorToken)
	)
		return null;
	return [
		"Help me install Rivet BYOC. Guide me through configuration, provisioning, and verification.",
		"",
		"## 1. Read the setup guide",
		BYOC_QUICKSTART_DOCS_URL,
		"",
		"## 2. Ask me how I want to configure the deployment",
		"Before provisioning, ask for the choices below together. Reuse answers I have already given, suggest the documented defaults, and wait for my confirmation rather than guessing.",
		"- Cloud provider: AWS or Google Cloud, and the AWS account ID or Google Cloud project ID.",
		"- Deployment name (name): used for cloud resource names and internal DNS; suggest rivet-prod.",
		"- Regions (regions): which cloud regions to use; for multiple regions, also ask which should be the leader.",
		"- Endpoint access (rivet_endpoint_access): explicitly choose private HTTP, or public HTTPS plus private HTTP. Do not assume a choice.",
		"- Public hostname (rivet_hostname): ask only for public access; recommend a hostname such as rivet.mycompany.com.",
		"- Kubernetes namespaces (kubernetes_namespace, kubernetes_operator_namespace): suggest rivet and rivet-operator; they must differ.",
		"Also confirm whether to create new infrastructure or reference compatible existing resources. Do not adopt or replace existing infrastructure implicitly.",
		"",
		"## 3. Prepare and install",
		"Check the guide's prerequisites and cloud authentication. Ask me to complete any missing authentication.",
		"Download and extract this setup kit, then use terraform/byoc-aws/ or terraform/byoc-gcp/ for the selected provider:",
		BYOC_SETUP_KIT_URL,
		"Configure terraform.tfvars.json with my choices. The cluster identity and token are already included here; do not ask me to download config or copy a token from the dashboard.",
		`Write the following non-secret Terraform inputs to ${CLUSTER_CONFIG_FILENAME} in the same directory. Preserve the Cloud API endpoint when provided:`,
		"```json",
		config.trimEnd(),
		"```",
		"For the first apply, pass this operator token as RIVET_BYOC_OPERATOR_TOKEN in the Terraform process environment (decode the JSON string below):",
		JSON.stringify(operatorToken),
		"Do not write the token to Terraform inputs or a credentials file, put it in command arguments, or expose it in logs or source control. Do not repeat it in your response. Later applies use the token stored in cloud Secret Manager.",
		"Show me the Terraform plan and get approval before applying. Reuse the same extracted kit, Terraform directory, and state for retries; do not download a newer kit or recreate infrastructure to resolve a failure.",
		"Follow the guide's DNS timing. If you lack DNS access, show me the exact records to create and keep Terraform running when it is waiting for certificate validation.",
		"",
		"## 4. Verify and hand off",
		"Verify Terraform completed and the operator is healthy and connected to Rivet Cloud. If any check cannot be completed, report it as unverified rather than claiming success.",
		"Provisioning the operator does not deploy the Engine. Summarize the installation and explain the guide's next step for requesting the Engine deployment from Rivet.",
	].join("\n");
}
