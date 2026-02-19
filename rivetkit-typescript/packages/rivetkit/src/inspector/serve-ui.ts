
import { extract } from "tar";
import { getNodeFs, getNodeOs, getNodePath, getNodeUrl } from "@/utils/node";

let extractedDir: string | undefined;
let extractionPromise: Promise<string> | undefined;

export async function getInspectorDir(): Promise<string> {
	if (extractedDir !== undefined) return extractedDir;
	if (extractionPromise !== undefined) return extractionPromise;

	const nodeFs = getNodeFs();
	const os = getNodeOs();
	const url = getNodeUrl();
	const path = getNodePath();

	extractionPromise = (async () => {
		const tarball = path.join(
			path.dirname(url.fileURLToPath(import.meta.url)),
			"../../dist/inspector.tar.gz",
		);

		try {
			await nodeFs.access(tarball);
		} catch {
			throw new Error(
				`Inspector tarball not found at ${tarball}. Run 'pnpm build:pack-inspector' first.`,
			);
		}

		const dest = path.join(os.tmpdir(), "rivetkit-inspector");
		await nodeFs.mkdir(dest, { recursive: true });
		await extract({ file: tarball, cwd: dest });

		extractedDir = dest;
		return dest;
	})();

	return extractionPromise;
}
