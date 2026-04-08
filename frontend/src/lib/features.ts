const envValue = import.meta.env.VITE_FEATURE_FLAGS as string | undefined;

const raw = import.meta.env.DEV
	? (localStorage.getItem("FEATURE_FLAGS") ?? envValue)
	: envValue;

// null means all flags are on (env var not set = full cloud build)
const enabled =
	raw === undefined
		? null
		: new Set(raw.split(",").map((s) => s.trim()).filter(Boolean));

function isEnabled(flag: string): boolean {
	return enabled === null || enabled.has(flag);
}

const auth = isEnabled("auth");
const multitenancy = isEnabled("multitenancy") && auth;

export const features = {
	auth,
	billing: isEnabled("billing"),
	support: isEnabled("support"),
	branding: isEnabled("branding"),
	datacenter: isEnabled("datacenter"),
	namespaceManagement: isEnabled("namespace-management"),
	multitenancy,
} as const;
