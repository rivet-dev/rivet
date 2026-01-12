import {
	faChevronRight,
	faNextjs,
	faNodeJs,
	faReact,
	Icon,
} from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { hasProvider } from "@/app/data-providers/engine-data-provider";
import { CodeFrame, CodeGroup, CodePreview } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";

interface PublishableTokenCodeGroupProps {
	token: string;
	endpoint: string;
	namespace: string;
}

export function PublishableTokenCodeGroup({
	token,
	endpoint,
	namespace,
}: PublishableTokenCodeGroupProps) {
	const dataProvider = useEngineCompatDataProvider();
	const { data: configs } = useInfiniteQuery({
		...dataProvider.runnerConfigsQueryOptions(),
		refetchInterval: 5000,
	});

	// Check if Vercel is connected
	const hasVercel = hasProvider(configs, ["vercel", "next-js"]);

	const nextJsTab = (
		<CodeFrame
			language="typescript"
			title="Next.js"
			icon={faNextjs}
			code={() => nextJsCode({ token, endpoint, namespace })}
			footer={
				<a
					href="https://rivet.dev/docs/actors/quickstart/next-js"
					target="_blank"
					rel="noopener noreferrer"
				>
					<span className="cursor-pointer hover:underline">
						See Next.js Documentation{" "}
						<Icon icon={faChevronRight} className="text-xs" />
					</span>
				</a>
			}
		>
			<CodePreview
				code={nextJsCode({ token, endpoint, namespace })}
				language="typescript"
			/>
		</CodeFrame>
	);

	const reactTab = (
		<CodeFrame
			language="typescript"
			title="React"
			icon={faReact}
			code={() => reactCode({ token, endpoint, namespace })}
			footer={
				<a
					href="https://rivet.dev/docs/actors/quickstart/react"
					target="_blank"
					rel="noopener noreferrer"
				>
					<span className="cursor-pointer hover:underline">
						See React Documentation{" "}
						<Icon icon={faChevronRight} className="text-xs" />
					</span>
				</a>
			}
		>
			<CodePreview
				code={reactCode({ token, endpoint, namespace })}
				language="typescript"
			/>
		</CodeFrame>
	);

	const javascriptTab = (
		<CodeFrame
			language="typescript"
			title="JavaScript"
			icon={faNodeJs}
			code={() =>
				javascriptCode({
					token,
					endpoint,
					namespace,
				})
			}
			footer={
				<a
					href="https://rivet.dev/docs/actors/quickstart/backend"
					target="_blank"
					rel="noopener noreferrer"
				>
					<span className="cursor-pointer hover:underline">
						See JavaScript Documentation{" "}
						<Icon icon={faChevronRight} className="text-xs" />
					</span>
				</a>
			}
		>
			<CodePreview
				code={javascriptCode({
					token,
					endpoint,
					namespace,
				})}
				language="typescript"
			/>
		</CodeFrame>
	);

	return (
		<CodeGroup>
			{hasVercel
				? [nextJsTab, reactTab, javascriptTab]
				: [javascriptTab, reactTab, nextJsTab]}
		</CodeGroup>
	);
}

const javascriptCode = ({
	token,
	endpoint,
	namespace,
}: {
	token: string;
	endpoint: string;
	namespace: string;
}) => `import { createClient } from "rivetkit/client";
import type { registry } from "./registry";

const client = createClient<typeof registry>({
	endpoint: "${endpoint}",
	namespace: "${namespace}",
	// This token is safe to publish on your frontend
	token: "${token}",
});`;

const reactCode = ({
	token,
	endpoint,
	namespace,
}: {
	token: string;
	endpoint: string;
	namespace: string;
}) => `import { createRivetKit } from "@rivetkit/react";
import type { registry } from "./registry";

export const { useActor } = createRivetKit<typeof registry>({
	endpoint: "${endpoint}",
	namespace: "${namespace}",
	// This token is safe to publish on your frontend
	token: "${token}",
});`;

const nextJsCode = ({
	token,
	endpoint,
	namespace,
}: {
	token: string;
	endpoint: string;
	namespace: string;
}) => `"use client";
import { createRivetKit } from "@rivetkit/next-js/client";
import type { registry } from "@/rivet/registry";

export const { useActor } = createRivetKit<typeof registry>({
	endpoint: "${endpoint}",
	namespace: "${namespace}",
	// This token is safe to publish on your frontend
	token: "${token}",
});`;
