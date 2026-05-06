import type { Story } from "@ladle/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "../../.ladle/ladle.css";
import { TooltipProvider } from "@/components";
import type { RivetActorError } from "@/queries/types";
import { RunnerPoolErrorPopover } from "./runner-pool-error-popover";

const queryClient = new QueryClient({
	defaultOptions: { queries: { retry: false, staleTime: Infinity } },
});

function Frame({ children }: { children: React.ReactNode }) {
	return (
		<QueryClientProvider client={queryClient}>
			<TooltipProvider>
				<div className="bg-background min-h-screen p-12">
					<div className="max-w-3xl space-y-8">{children}</div>
				</div>
			</TooltipProvider>
		</QueryClientProvider>
	);
}

const HTTP_500_BODY = JSON.stringify(
	{
		error: "InternalServerError",
		message:
			"Cannot read properties of undefined (reading 'invoke')\n    at /var/task/index.js:142:18\n    at processTicksAndRejections (node:internal/process/task_queues:96:5)",
		requestId: "8f1c4a2e-9b3d-4e1a-a5f7-d2c3e4b5a6f9",
	},
	null,
	2,
);

const HTTP_500: RivetActorError = {
	serverless_http_error: { status_code: 500, body: HTTP_500_BODY },
};

const HTTP_502: RivetActorError = {
	serverless_http_error: {
		status_code: 502,
		body: "Bad Gateway: upstream connection refused",
	},
};

const CONN_ERROR: RivetActorError = {
	serverless_connection_error: {
		message:
			"dial tcp 10.0.4.21:443: connect: connection timed out after 30s",
	},
};

const SSE_INVALID: RivetActorError = {
	serverless_invalid_sse_payload: {
		message:
			"Expected actor_id field in SSE event payload, got: { \"type\": \"start\" }",
	},
};

function Section({
	title,
	children,
}: {
	title: string;
	children: React.ReactNode;
}) {
	return (
		<div className="space-y-2">
			<h3 className="text-sm font-medium text-muted-foreground">
				{title}
			</h3>
			<div className="rounded-md border bg-card p-4">{children}</div>
		</div>
	);
}

export const SingleRegionHttpError: Story = () => (
	<Frame>
		<Section title="Single region · HTTP 500">
			<RunnerPoolErrorPopover errors={{ "us-east-1": HTTP_500 }} />
		</Section>
	</Frame>
);

export const MultipleRegionsSameError: Story = () => (
	<Frame>
		<Section title="Same root cause across 3 regions (grouped)">
			<RunnerPoolErrorPopover
				errors={{
					"us-east-1": HTTP_500,
					"us-west-2": HTTP_500,
					"eu-central-1": HTTP_500,
				}}
			/>
		</Section>
	</Frame>
);

export const MultipleRegionsDifferentErrors: Story = () => (
	<Frame>
		<Section title="Mixed errors across regions (tabs)">
			<RunnerPoolErrorPopover
				errors={{
					"us-east-1": HTTP_500,
					"us-west-2": CONN_ERROR,
					"eu-central-1": HTTP_502,
					"ap-southeast-1": SSE_INVALID,
				}}
			/>
		</Section>
	</Frame>
);

export const ConnectionError: Story = () => (
	<Frame>
		<Section title="Connection failure">
			<RunnerPoolErrorPopover errors={{ "us-east-1": CONN_ERROR }} />
		</Section>
	</Frame>
);

export const WarningOnly: Story = () => (
	<Frame>
		<Section title="Warning severity (SSE payload)">
			<RunnerPoolErrorPopover
				errors={{
					"us-east-1": SSE_INVALID,
					"us-west-2": SSE_INVALID,
				}}
			/>
		</Section>
	</Frame>
);

export const StringError: Story = () => (
	<Frame>
		<Section title="String error · downgrade (warning)">
			<RunnerPoolErrorPopover
				errors={{ "us-east-1": "downgrade" as RivetActorError }}
			/>
		</Section>
	</Frame>
);

export const IconOnlyTrigger: Story = () => (
	<Frame>
		<Section title="Icon-only trigger (sidebar Settings indicator)">
			<div className="flex items-center gap-2 text-sm text-muted-foreground">
				<span>Settings</span>
				<RunnerPoolErrorPopover
					iconOnly
					errors={{
						"us-east-1": HTTP_500,
						"us-west-2": CONN_ERROR,
					}}
				/>
			</div>
		</Section>
		<Section title="Icon-only · single warning">
			<div className="flex items-center gap-2 text-sm text-muted-foreground">
				<span>Settings</span>
				<RunnerPoolErrorPopover
					iconOnly
					errors={{ "us-east-1": SSE_INVALID }}
				/>
			</div>
		</Section>
	</Frame>
);

export const WithEditAction: Story = () => (
	<Frame>
		<Section title="With Edit runner config action">
			<RunnerPoolErrorPopover
				errors={{
					"us-east-1": HTTP_500,
					"us-west-2": CONN_ERROR,
				}}
				onEditConfig={() => alert("Navigate to runner config")}
			/>
		</Section>
	</Frame>
);

export const Gallery: Story = () => (
	<Frame>
		<Section title="HTTP 500 · 1 region">
			<RunnerPoolErrorPopover errors={{ "us-east-1": HTTP_500 }} />
		</Section>
		<Section title="HTTP 500 · 3 regions (grouped)">
			<RunnerPoolErrorPopover
				errors={{
					"us-east-1": HTTP_500,
					"us-west-2": HTTP_500,
					"eu-central-1": HTTP_500,
				}}
			/>
		</Section>
		<Section title="Mixed errors · 4 regions (tabs)">
			<RunnerPoolErrorPopover
				errors={{
					"us-east-1": HTTP_500,
					"us-west-2": CONN_ERROR,
					"eu-central-1": HTTP_502,
					"ap-southeast-1": SSE_INVALID,
				}}
				onEditConfig={() => {}}
			/>
		</Section>
		<Section title="Warning only">
			<RunnerPoolErrorPopover
				errors={{ "us-east-1": SSE_INVALID, "us-west-2": SSE_INVALID }}
			/>
		</Section>
	</Frame>
);
