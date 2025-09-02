import $ from "dax";

const REPOS = [
	{ name: "rivetgg/rivet-server", prefix: "slim", main: true },
	{ name: "rivetgg/rivet-server", prefix: "full" },
	{ name: "rivetgg/rivet-client", prefix: "full", main: true },
	{ name: "rivetgg/rivet-client", prefix: "container-runner" },
	//{ name: "rivetgg/rivet", prefix: "monolith", main: true },
]

export async function tagDocker(opts: { version: string; commit: string; latest: boolean }) {
	for (const { name, prefix, main } of REPOS) {
		// Check image exists
		$.logStep("Pulling", `${name}:${prefix}-${opts.commit}`);
		const imageExists = await $`docker pull --platform amd64 ${name}:${prefix}-${opts.commit}`.quiet().noThrow();
		if (imageExists.code !== 0) {
			throw new Error(`Image ${name}:${prefix}-${opts.commit} does not exist on Docker Hub.`);
		}

		// Tag with version
		await tag(name, `${prefix}-${opts.commit}`, `${prefix}-${opts.version}`);
		if (main) {
			await tag(name, `${prefix}-${opts.commit}`, opts.version);
		}

		// Tag with latest
		if (opts.latest) {
			await tag(name, `${prefix}-${opts.commit}`, `${prefix}-latest`);
			if (main) {
				await tag(name, `${prefix}-${opts.commit}`, "latest");
			}
		}
	}
}

async function tag(image: string, from: string, to: string) {
	$.logStep("Tagging", `${image}:${from} -> ${image}:${to}`);
	await $`docker tag ${image}:${from} ${image}:${to}`;
	await $`docker push ${image}:${to}`;
}
