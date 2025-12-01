import {
	faChevronRight,
	faNextjs,
	faNodeJs,
	faReact,
	Icon,
} from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { hasProvider } from "@/app/data-providers/engine-data-provider";
import { useRivetDsn } from "@/app/env-variables";
import { CodeFrame, CodeGroup, CodePreview } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";

export function PublishableTokenCodeGroup() {
	const dataProvider = useEngineCompatDataProvider();
	const { data: configs } = useInfiniteQuery({
		...dataProvider.runnerConfigsQueryOptions(),
		refetchInterval: 5000,
	});

	// Check if Vercel is connected
	const hasVercel = hasProvider(configs, ["vercel", "next-js"]);

	const endpoint = useRivetDsn({ endpoint: "", kind: "publishable" });

	const nextJsTab = (
		<CodeFrame
			language="typescript"
			title="Next.js"
			icon={faNextjs}
			code={() => nextJsCode(endpoint)}
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
			<CodePreview code={nextJsCode(endpoint)} language="typescript" />
		</CodeFrame>
	);

	const reactTab = (
		<CodeFrame
			language="typescript"
			title="React"
			icon={faReact}
			code={() => reactCode(endpoint)}
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
			<CodePreview code={reactCode(endpoint)} language="typescript" />
		</CodeFrame>
	);

	const javascriptTab = (
		<CodeFrame
			language="typescript"
			title="JavaScript"
			icon={faNodeJs}
			code={() => javascriptCode(endpoint)}
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
				code={javascriptCode(endpoint)}
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

const javascriptCode = (
	endpoint: string,
) => `import { createClient } from "rivetkit/client";
import type { registry } from "./registry";

const client = createClient<typeof registry>("${endpoint}");`;

const reactCode = (
	endpoint: string,
) => `import { createRivetKit } from "@rivetkit/react";
import type { registry } from "./registry";

export const { useActor } = createRivetKit<typeof registry>("${endpoint}");`;

const nextJsCode = (endpoint: string) => `"use client";
import { createRivetKit } from "@rivetkit/next-js/client";
import type { registry } from "@/rivet/registry";

export const { useActor } = createRivetKit<typeof registry>("${endpoint}");`;
