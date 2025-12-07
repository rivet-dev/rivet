export const askForLocalNetworkAccess = async () => {
	try {
		const status = await navigator.permissions.query({
			// @ts-expect-error missing types
			name: "local-network-access",
		});
		if (status.state === "granted") {
			return true;
		}
		if (status.state === "denied") {
			return false;
		}
		// If promptable, try to request permission
		if (status.state === "prompt") {
			// Note: There is currently no way to programmatically trigger the permission prompt.
			// It is triggered when the app tries to access local network resources.
			// So we return true here and handle any failures when trying to connect.
			return true;
		}
	} catch {
		// Permissions API not supported
		return true;
	}
};
