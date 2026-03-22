import { CodeBlock } from "../CodeBlock";
import { Code, CodeGroup } from "../mdx";

interface StepRunServerProps {
	file?: string;
	showDescription?: boolean;
}

export function StepRunServer({ file = "index.ts", showDescription = true }: StepRunServerProps) {
	return (
		<>
			<CodeGroup>
				<Code title="Node.js" language="bash">
					<CodeBlock lang="bash" code={`npx tsx --watch ${file}`} />
				</Code>
				<Code title="Bun" language="bash">
					<CodeBlock lang="bash" code={`bun --watch ${file}`} />
				</Code>
				<Code title="Deno" language="bash">
					<CodeBlock lang="bash" code={`deno run --allow-net --allow-read --allow-env --watch ${file}`} />
				</Code>
			</CodeGroup>

			{showDescription && (
				<p>Your server is now running. See <a href="/docs/general/http-server">Server Setup</a> for runtime-specific configurations.</p>
			)}
		</>
	);
}
