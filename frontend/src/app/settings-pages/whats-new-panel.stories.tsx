import type { Story } from "@ladle/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "../../../.ladle/ladle.css";
import type { Changelog } from "@/queries/types";
import { WhatsNewPanel } from "./whats-new-panel";

const sampleEntries: Changelog = [
	{
		published: "2025-09-28T00:00:00.000Z",
		title: "Weekly Updates",
		description:
			"Performance improvements, new lifecycle hooks, and a refreshed dashboard navigation experience for managing your Rivet Actors.",
		slug: "2025-09-28-weekly-updates",
		images: [
			{
				url: "images/changelog/2025-09-28-weekly-updates.png",
				width: 1200,
				height: 600,
			},
		],
		authors: [
			{
				name: "Nathan Flurry",
				role: "Co-founder",
				avatar: { url: "images/authors/nathan-flurry.png" },
				socials: { twitter: "https://twitter.com/nathanflurry" },
			},
		],
	},
	{
		published: "2025-09-21T00:00:00.000Z",
		title: "Faster cold starts",
		description:
			"We've cut p99 cold start latency by 40% across all regions thanks to new prewarming logic in the runner pool.",
		slug: "2025-09-21-weekly-updates",
		images: [
			{
				url: "images/changelog/2025-09-21-weekly-updates.png",
				width: 1200,
				height: 600,
			},
		],
		authors: [
			{
				name: "NicoletaMera",
				role: "Engineer",
				avatar: { url: "images/authors/nicoleta-mera.png" },
				socials: {},
			},
		],
	},
];

type FrameState = "success" | "loading" | "empty" | "error";

function Frame({ state }: { state: FrameState }) {
	const client = new QueryClient({
		defaultOptions: { queries: { retry: false, staleTime: Infinity } },
	});

	if (state === "success") {
		client.setQueryDefaults(["changelog"], {
			queryFn: () => Promise.resolve(sampleEntries),
		});
	} else if (state === "empty") {
		client.setQueryDefaults(["changelog"], {
			queryFn: () => Promise.resolve([] as Changelog),
		});
	} else if (state === "error") {
		client.setQueryDefaults(["changelog"], {
			queryFn: () => Promise.reject(new Error("boom")),
		});
	} else if (state === "loading") {
		client.setQueryDefaults(["changelog"], {
			queryFn: () => new Promise<Changelog>(() => {}),
		});
	}

	return (
		<QueryClientProvider client={client}>
			<div className="bg-background min-h-screen p-6">
				<div className="mx-auto max-w-2xl">
					<WhatsNewPanel />
				</div>
			</div>
		</QueryClientProvider>
	);
}

export const Success: Story = () => <Frame state="success" />;
export const Loading: Story = () => <Frame state="loading" />;
export const Empty: Story = () => <Frame state="empty" />;
export const ErrorState: Story = () => <Frame state="error" />;

export const Gallery: Story = () => (
	<div className="bg-background min-h-screen p-6 space-y-8">
		{(["success", "loading", "empty", "error"] as FrameState[]).map(
			(state) => (
				<div key={state}>
					<h2 className="text-foreground text-sm font-semibold mb-2 capitalize">
						{state}
					</h2>
					<div className="max-w-2xl">
						<Frame state={state} />
					</div>
				</div>
			),
		)}
	</div>
);
