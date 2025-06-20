// Keep in sync with ci-runner/entry.sh
export const UNIT_SEP_CHAR = "\x1F";
export const NO_SEP_CHAR_REGEX = /^[^\x1F]+$/;

interface KanikoArguments {
	contextUrl: string;
	outputUrl: string;
	destination: string;
	dockerfilePath: string;
	buildArgs: Record<string, string>;
	buildTarget?: string;
}

// SAFETY: buildArgs keys never have equal signs or spaces
function convertBuildArgsToArgs(
	buildArgs: Record<string, string>,
): string[] {
	return Object.entries(buildArgs).flatMap(([key, value]) => [
		`--build-arg`,
		`${key}=${value}`,
	]);
}

export function serializeKanikoArguments(args: KanikoArguments): string {
	// SAFETY: Nothing needed to be escaped, as values are already sanitized,
	// and are joined by IFS=UNIT_SEP_CHAR (see entry.sh of ci-runner).
	const preparedArgs = [
		...convertBuildArgsToArgs(args.buildArgs),
		`--context=${args.contextUrl}`,
		`--destination=${args.destination}`,
		`--upload-tar=${args.outputUrl}`,
		`--dockerfile=${args.dockerfilePath}`,
		...(args.buildTarget ? [`--target='${args.buildTarget}'`] : []),
		"--no-push",
		"--single-snapshot",
		"--verbosity=info",
	].map(arg => {
		// Args should never contain UNIT_SEP_CHAR, but we can
		// escape it if they do.
		return arg.replaceAll(UNIT_SEP_CHAR, "\\" + UNIT_SEP_CHAR)
	});

	return preparedArgs.join(UNIT_SEP_CHAR);
}