export const BYOC_DOCS_URL = "https://rivet.dev/cloud/byoc/";
export const BYOC_QUICKSTART_DOCS_URL =
	"https://rivet.dev/cloud/byoc/quickstart/";
export const BYOC_SETUP_KIT_URL =
	"https://releases.rivet.dev/byoc/latest/setup-kit.tar.gz";
export const BYOC_CAL_NAMESPACE = "byoc";
export const BYOC_CAL_LINK = "team/rivet/byoc";
export const BYOC_SUPPORT_EMAIL = "support@rivet.dev";

export const BYOC_TRIAL_DAYS = 14;

/** BYOC plan card copy; the cloud plan catalog lives in `billing.ts`. */
export const BYOC_PLAN = {
	price: "Custom",
	description: "A fully-managed Rivet cluster inside your own cloud account.",
	rows: [
		{ label: "Runs in your VPC" },
		{ label: "Operated by Rivet" },
		{ label: "Data residency" },
		{ label: "CMEK for PCI & HIPAA" },
		{ label: "Enterprise support" },
		{ label: `${BYOC_TRIAL_DAYS}-day free trial` },
	],
} as const;
