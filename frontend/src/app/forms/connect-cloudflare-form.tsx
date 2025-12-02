import z from "zod";
import * as ConnectManualServerlessForm from "@/app/forms/connect-manual-serverless-form";
import {
	CodeFrame,
	CodeGroup,
	CodePreview,
} from "@/components";
import { defineStepper } from "@/components/ui/stepper";

const endpointSchema = z
	.string()
	.nonempty("Endpoint is required")
	.url("Please enter a valid URL")
	.endsWith("/api/rivet", "Endpoint must end with /api/rivet");

export const stepper = defineStepper(
	{
		id: "install",
		title: "Install RivetKit",
		assist: false,
		schema: z.object({}),
		next: "Next",
	},
	{
		id: "configure",
		title: "Configure Wrangler",
		assist: false,
		schema: z.object({}),
		next: "Next",
	},
	{
		id: "handler",
		title: "Create Handler",
		assist: false,
		schema: z.object({}),
		next: "Next",
	},
	{
		id: "deploy",
		title: "Deploy to Cloudflare",
		assist: true,
		next: "Done",
		schema: z.object({
			success: z.boolean().refine((val) => val, "Connection failed"),
			endpoint: endpointSchema,
			runnerName: z.string().min(1, "Runner name is required"),
			datacenters: z
				.record(z.boolean())
				.refine(
					(data) => Object.values(data).some(Boolean),
					"At least one datacenter must be selected",
				),
			headers: z.array(z.tuple([z.string(), z.string()])).default([]),
			slotsPerRunner: z.coerce.number().min(1, "Must be at least 1"),
			maxRunners: z.coerce.number().min(1, "Must be at least 1"),
			minRunners: z.coerce.number().min(0, "Must be 0 or greater"),
			runnerMargin: z.coerce.number().min(0, "Must be 0 or greater"),
		}),
	},
);

export const RunnerName = ConnectManualServerlessForm.RunnerName;

export const Datacenters = ConnectManualServerlessForm.Datacenters;

export const MinRunners = ConnectManualServerlessForm.MinRunners;

export const MaxRunners = ConnectManualServerlessForm.MaxRunners;

export const SlotsPerRunner = ConnectManualServerlessForm.SlotsPerRunner;

export const RunnerMargin = ConnectManualServerlessForm.RunnerMargin;

export const Headers = ConnectManualServerlessForm.Headers;

export const Endpoint = ConnectManualServerlessForm.Endpoint;

export const ConnectionCheck = ConnectManualServerlessForm.ConnectionCheck;

export const InstallCode = () => {
	return (
		<div className="space-y-4 mt-2">
			<p>
				If you have not created a project, see the{" "}
				<a
					href="https://www.rivet.dev/docs/actors/quickstart/cloudflare-workers/"
					target="_blank"
					rel="noopener noreferrer"
					className="underline hover:text-foreground"
				>
					Cloudflare Workers quickstart guide
				</a>
				.
			</p>
			<p>Install the RivetKit Cloudflare Workers package:</p>
			<CodeGroup>
				<CodeFrame
					language="bash"
					title="npm"
					code={() => "npm install rivetkit @rivetkit/cloudflare-workers"}
				>
					<CodePreview
						className="w-full min-w-0"
						language="bash"
						code="npm install rivetkit @rivetkit/cloudflare-workers"
					/>
				</CodeFrame>
				<CodeFrame
					language="bash"
					title="pnpm"
					code={() => "pnpm add rivetkit @rivetkit/cloudflare-workers"}
				>
					<CodePreview
						className="w-full min-w-0"
						language="bash"
						code="pnpm add rivetkit @rivetkit/cloudflare-workers"
					/>
				</CodeFrame>
				<CodeFrame
					language="bash"
					title="yarn"
					code={() => "yarn add rivetkit @rivetkit/cloudflare-workers"}
				>
					<CodePreview
						className="w-full min-w-0"
						language="bash"
						code="yarn add rivetkit @rivetkit/cloudflare-workers"
					/>
				</CodeFrame>
				<CodeFrame
					language="bash"
					title="bun"
					code={() => "bun add rivetkit @rivetkit/cloudflare-workers"}
				>
					<CodePreview
						className="w-full min-w-0"
						language="bash"
						code="bun add rivetkit @rivetkit/cloudflare-workers"
					/>
				</CodeFrame>
			</CodeGroup>
		</div>
	);
};

const wranglerConfig = `{
	"name": "my-rivetkit-app",
	"main": "src/index.ts",
	"compatibility_date": "2025-01-20",
	"compatibility_flags": ["nodejs_compat"],
	"migrations": [
		{
			"tag": "v1",
			"new_sqlite_classes": ["ActorHandler"]
		}
	],
	"durable_objects": {
		"bindings": [
			{
				"name": "ACTOR_DO",
				"class_name": "ActorHandler"
			}
		]
	},
	"kv_namespaces": [
		{
			"binding": "ACTOR_KV",
			"id": "YOUR_KV_NAMESPACE_ID"
		}
	],
	"observability": {
		"enabled": true
	}
}`;

export const WranglerConfig = () => {
	return (
		<div className="space-y-4 mt-2">
			<p>
				Configure your <code className="text-sm bg-muted px-1 py-0.5 rounded">wrangler.json</code> with the required Durable Objects and KV bindings:
			</p>
			<CodeFrame
				language="json"
				title="wrangler.json"
				code={() => wranglerConfig}
			>
				<CodePreview
					className="w-full min-w-0"
					language="json"
					code={wranglerConfig}
				/>
			</CodeFrame>
			<p className="text-muted-foreground text-sm">
				You'll need to create a KV namespace using{" "}
				<code className="text-xs bg-muted px-1 py-0.5 rounded">wrangler kv namespace create ACTOR_KV</code>{" "}
				and replace <code className="text-xs bg-muted px-1 py-0.5 rounded">YOUR_KV_NAMESPACE_ID</code> with the generated ID.
			</p>
		</div>
	);
};

const handlerCode = `import { createHandler } from "@rivetkit/cloudflare-workers";
import { registry } from "./registry";

const { handler, ActorHandler } = createHandler(registry);
export { handler as default, ActorHandler };`;

const registryCode = `import { actor, setup } from "rivetkit";

export const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, x: number) => {
			c.state.count += x;
			c.broadcast("newCount", c.state.count);
			return c.state.count;
		},
	},
});

export const registry = setup({
	use: { counter },
});`;

export const HandlerCode = () => {
	return (
		<div className="space-y-4 mt-2">
			<p>Create your actor registry:</p>
			<CodeFrame
				language="typescript"
				title="src/registry.ts"
				code={() => registryCode}
			>
				<CodePreview
					className="w-full min-w-0"
					language="typescript"
					code={registryCode}
				/>
			</CodeFrame>
			<p>Create the Cloudflare Workers handler:</p>
			<CodeFrame
				language="typescript"
				title="src/index.ts"
				code={() => handlerCode}
			>
				<CodePreview
					className="w-full min-w-0"
					language="typescript"
					code={handlerCode}
				/>
			</CodeFrame>
		</div>
	);
};
