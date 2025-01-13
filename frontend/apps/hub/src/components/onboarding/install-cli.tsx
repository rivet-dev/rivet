import { CodeFrame, CodeGroup, CodeSource } from "@rivet-gg/components";
import installCliCmd, {
	source as installCliCmdSource,
} from "./initial-setup-install-rivet-cli-cmd.sh?shiki&lang=bash";
import installCliPowerShell, {
	source as installCliPowerShellSource,
} from "./initial-setup-install-rivet-cli-powershell.sh?shiki&lang=bash";
import installCliSource, {
	source as installCliSourceSource,
} from "./initial-setup-install-rivet-cli-source.sh?shiki&lang=bash";
import installCliUnix, {
	source as installCliUnixSource,
} from "./initial-setup-install-rivet-cli-unix.sh?shiki&lang=bash";

export function InstallCli() {
	return (
		<CodeGroup>
			<CodeFrame
				title="macOS & Linux & WSL"
				code={installCliUnixSource}
				language="bash"
			>
				<CodeSource>{installCliUnix}</CodeSource>
			</CodeFrame>
			<CodeFrame
				title="Windows (cmd)"
				code={installCliCmdSource}
				language="ps1"
			>
				<CodeSource>{installCliCmd}</CodeSource>
			</CodeFrame>
			<CodeFrame
				title="Windows (PowerShell)"
				code={installCliPowerShellSource}
				language="powershell"
			>
				<CodeSource>{installCliPowerShell}</CodeSource>
			</CodeFrame>
			<CodeFrame
				title="Build from source"
				code={installCliSourceSource}
				language="bash"
			>
				<CodeSource>{installCliSource}</CodeSource>
			</CodeFrame>
		</CodeGroup>
	);
}
