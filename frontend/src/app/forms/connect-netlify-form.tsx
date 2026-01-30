import { useFormContext, useWatch } from "react-hook-form";
import { useState } from "react";
import z from "zod";
import * as ConnectManualServerlessForm from "@/app/forms/connect-manual-serverless-form";
import {
	CodeFrame,
	CodeGroup,
	CodePreview,
	FormControl,
	FormDescription,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components";
import { defineStepper } from "@/components/ui/stepper";
import { useEndpoint } from "../dialogs/connect-manual-serverfull-frame";
import {
	EnvVariables as EnvVariablesSection,
	useRivetDsn,
} from "../env-variables";

export const stepper = defineStepper(
	{
		id: "api-route",
		title: "Add API Route",
		assist: false,
		schema: z.object({
			framework: z.string().min(1, "Please select a framework"),
		}),
		next: "Next",
	},
	{
		id: "frontend",
		title: "Connect Frontend",
		assist: false,
		next: "Next",
		optional: true,
		schema: z.object({}),
	},
	{
		id: "variables",
		title: "Configure Environment Variables",
		assist: false,
		next: "Next",
		optional: true,
		schema: z.object({}),
	},
	{
		id: "deploy",
		title: "Deploy to Netlify",
		assist: true,
		next: "Done",
		schema: z.object({
			...ConnectManualServerlessForm.deploymentSchema.shape,
			...ConnectManualServerlessForm.configurationSchema.omit({
				requestLifespan: true,
			}).shape,
			plan: z.string().min(1, "Please select a Netlify plan"),
		}),
	},
);

export const Plan = ({ className }: { className?: string }) => {
	const { control } = useFormContext();
	return (
		<FormField
			control={control}
			name="plan"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Netlify Plan</FormLabel>
					<FormControl className="row-start-2">
						<Select
							onValueChange={field.onChange}
							value={field.value}
						>
							<SelectTrigger>
								<SelectValue placeholder="Select your Netlify plan..." />
							</SelectTrigger>
							<SelectContent>
								<SelectItem value="starter">Starter</SelectItem>
								<SelectItem value="pro">Pro</SelectItem>
								<SelectItem value="business">Business</SelectItem>
								<SelectItem value="enterprise">
									Enterprise
								</SelectItem>
							</SelectContent>
						</Select>
					</FormControl>
					<FormDescription className="col-span-1">
						Your Netlify plan determines the configuration required
						to properly connect Rivet to Netlify Functions.
					</FormDescription>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const RunnerName = ConnectManualServerlessForm.RunnerName;

export const Datacenters = ConnectManualServerlessForm.Datacenters;

export const MinRunners = ConnectManualServerlessForm.MinRunners;

export const MaxRunners = ConnectManualServerlessForm.MaxRunners;

export const SlotsPerRunner = ConnectManualServerlessForm.SlotsPerRunner;

export const RunnerMargin = ConnectManualServerlessForm.RunnerMargin;

export const Headers = ConnectManualServerlessForm.Headers;

const netlifyFunctionCode = () => {
	// Avoid nested template literals to prevent Babel parser issues
	const codeLines = [
		'import type { Handler } from "@netlify/functions";',
		'import app from "../src/server.ts";',
		'',
		'export const handler: Handler = async (event, context) => {',
		'  const { httpMethod, path, queryStringParameters, headers, body } = event;',
		'  ',
		'  // Convert Netlify event to standard Request',
		'  const url = `https://${headers.host}${path}${',
		'    queryStringParameters ',
		'      ? "?" + new URLSearchParams(queryStringParameters).toString() ',
		'      : ""',
		'  }`;',
		'  ',
		'  const request = new Request(url, {',
		'    method: httpMethod,',
		'    headers: headers as HeadersInit,',
		'    body: body ? body : undefined,',
		'  });',
		'',
		'  const response = await app.fetch(request);',
		'  ',
		'  // Convert Response to Netlify format',
		'  const responseHeaders: Record<string, string> = {};',
		'  response.headers.forEach((value, key) => {',
		'    responseHeaders[key] = value;',
		'  });',
		'',
		'  return {',
		'    statusCode: response.status,',
		'    headers: responseHeaders,',
		'    body: await response.text(),',
		'  };',
		'};'
	];
	return codeLines.join('\n');
};

const nextJsIntegrationCode = (plan: string) => {
	const maxDuration = plan === "starter" ? 10 : 26;
	const codeLines = [
		'import { toNextHandler } from "@rivetkit/next-js";',
		'import { registry } from "@/rivet/registry";',
		'',
		`export const maxDuration = ${maxDuration};`,
		'',
		'export const { GET, POST, PUT, PATCH, HEAD, OPTIONS } = toNextHandler(registry);'
	];
	return codeLines.join('\n');
};

export const IntegrationCode = ({ plan }: { plan: string }) => {
	const { control } = useFormContext();
	const framework = useWatch({ control, name: "framework" }) || "netlify-functions";

	return (
		<div className="space-y-4 mt-2">
			<FormField
				control={control}
				name="framework"
				render={({ field }) => (
					<FormItem>
						<FormLabel>Framework</FormLabel>
						<FormControl>
							<Select
								onValueChange={field.onChange}
								value={field.value || "netlify-functions"}
							>
								<SelectTrigger>
									<SelectValue placeholder="Select framework..." />
								</SelectTrigger>
								<SelectContent>
									<SelectItem value="netlify-functions">Netlify Functions</SelectItem>
									<SelectItem value="next-js">Next.js</SelectItem>
								</SelectContent>
							</Select>
						</FormControl>
						<FormMessage />
					</FormItem>
				)}
			/>

			{framework === "next-js" && (
				<>
					<p>
						If you have not created a project, see the{" "}
						<a
							href="https://www.rivet.dev/docs/actors/quickstart/next-js/"
							target="_blank"
							rel="noopener noreferrer"
							className="underline hover:text-foreground"
						>
							Next.js quickstart guide
						</a>
						.
					</p>
					<p>First, install the required packages:</p>
					<CodeGroup>
						<CodeFrame
							language="bash"
							title="npm"
							code={() => "npm install @rivetkit/next-js @netlify/plugin-nextjs"}
						>
							<CodePreview
								className="w-full min-w-0"
								language="bash"
								code="npm install @rivetkit/next-js @netlify/plugin-nextjs"
							/>
						</CodeFrame>
						<CodeFrame
							language="bash"
							title="pnpm"
							code={() => "pnpm add @rivetkit/next-js @netlify/plugin-nextjs"}
						>
							<CodePreview
								className="w-full min-w-0"
								language="bash"
								code="pnpm add @rivetkit/next-js @netlify/plugin-nextjs"
							/>
						</CodeFrame>
					</CodeGroup>
					<p>Then, add your Rivet route handler:</p>
					<CodeFrame
						language="typescript"
						title="src/app/api/rivet/[...all]/route.ts"
						code={() => nextJsIntegrationCode(plan)}
					>
						<CodePreview
							className="w-full min-w-0"
							language="typescript"
							code={nextJsIntegrationCode(plan)}
						/>
					</CodeFrame>
				</>
			)}

			{framework === "netlify-functions" && (
				<>
					<p>First, install the Netlify Functions package:</p>
					<CodeGroup>
						<CodeFrame
							language="bash"
							title="npm"
							code={() => "npm install @netlify/functions"}
						>
							<CodePreview
								className="w-full min-w-0"
								language="bash"
								code="npm install @netlify/functions"
							/>
						</CodeFrame>
						<CodeFrame
							language="bash"
							title="pnpm"
							code={() => "pnpm add @netlify/functions"}
						>
							<CodePreview
								className="w-full min-w-0"
								language="bash"
								code="pnpm add @netlify/functions"
							/>
						</CodeFrame>
					</CodeGroup>
					<p>Then, create your Netlify function handler:</p>
					<CodeFrame
						language="typescript"
						title="functions/rivet.ts"
						code={() => netlifyFunctionCode()}
					>
						<CodePreview
							className="w-full min-w-0"
							language="typescript"
							code={netlifyFunctionCode()}
						/>
					</CodeFrame>
					<p>And add a netlify.toml configuration:</p>
					<CodeFrame
						language="toml"
						title="netlify.toml"
						code={() => `[build]
  functions = "functions"
  publish = "dist"

[functions]
  node_bundler = "esbuild"

[[redirects]]
  from = "/api/rivet/*"
  to = "/.netlify/functions/rivet"
  status = 200`}
					>
						<CodePreview
							className="w-full min-w-0"
							language="toml"
							code={`[build]
  functions = "functions"
  publish = "dist"

[functions]
  node_bundler = "esbuild"

[[redirects]]
  from = "/api/rivet/*"
  to = "/.netlify/functions/rivet"
  status = 200`}
						/>
					</CodeFrame>
				</>
			)}
		</div>
	);
};

export const Endpoint = ConnectManualServerlessForm.Endpoint;

export const ConnectionCheck = ConnectManualServerlessForm.ConnectionCheck;

export const FrontendIntegrationCode = () => {
	const framework = useWatch({ name: "framework" }) || "netlify-functions";
	const endpoint = useRivetDsn({
		endpoint: useEndpoint(),
		kind: "publishable",
	});

	const clientPackage = framework === "next-js" ? "@rivetkit/next-js/client" : "@rivetkit/react";
	const clientCode = `"use client";
import { createRivetKit } from "${clientPackage}";
import type { registry } from "@/rivet/registry";

export const { useActor } = createRivetKit<typeof registry>("${endpoint}");
`;

	return (
		<div className="space-y-4 mt-2">
			<p>Connect your frontend to Rivet:</p>
			<CodeFrame
				language="typescript"
				title="src/lib/rivet.ts"
				code={() => clientCode}
			>
				<CodePreview
					className="w-full min-w-0"
					language="typescript"
					code={clientCode}
				/>
			</CodeFrame>
			<p className="text-muted-foreground text-sm">
				This token is safe to publish on your frontend.
			</p>
		</div>
	);
};

export function EnvVariables() {
	return (
		<EnvVariablesSection
			endpoint={useEndpoint()}
			runnerName={useWatch({ name: "runnerName" }) as string}
		/>
	);
}