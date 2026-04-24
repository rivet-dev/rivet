import { getNodeFs, getNodePath, getNodeUrl } from "@/utils/node";

export async function getDevtoolsPath(): Promise<string> {
	const url = getNodeUrl();
	const path = getNodePath();

	const devtoolsPath = path.join(
		path.dirname(url.fileURLToPath(import.meta.url)),
		"../../dist/devtools/mod.js",
	);

	try {
		await getNodeFs().access(devtoolsPath);
	} catch {
		throw new Error(
			`Devtools bundle not found at ${devtoolsPath}. Run 'pnpm build:pack-devtools' first.`,
		);
	}

	return devtoolsPath;
}

export async function readDevtoolsBundle(): Promise<Buffer> {
	const devtoolsPath = await getDevtoolsPath();
	return getNodeFs().readFile(devtoolsPath);
}
