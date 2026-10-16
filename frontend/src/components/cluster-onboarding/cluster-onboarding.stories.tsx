import type { Story } from "@ladle/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import "../../../.ladle/ladle.css";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import {
	CHAT_DESCRIPTION,
	ClusterOnboardingFlow,
	DashboardHarness,
} from "./cluster-onboarding-flow";
import { CreateClusterPage } from "./create-cluster-page";
import { DEFAULT_SELECTION } from "./model";

// Design mock of the new cluster onboarding (see cluster-onboarding-flow.tsx).
// Also mounted in the dashboard at /mock/cluster-onboarding in dev.

const queryClient = new QueryClient({
	defaultOptions: { queries: { retry: false, staleTime: Infinity } },
});

function Frame({ children }: { children: ReactNode }) {
	return (
		<QueryClientProvider client={queryClient}>
			<TooltipProvider>
				<div className="min-h-screen bg-background text-foreground">
					{children}
				</div>
				<Toaster />
			</TooltipProvider>
		</QueryClientProvider>
	);
}

export const CreateCluster: Story = () => (
	<Frame>
		<CreateClusterPage onCreate={(s) => console.log("create", s)} />
	</Frame>
);

export const CreateClusterByoc: Story = () => (
	<Frame>
		<CreateClusterPage
			defaultValues={{
				...DEFAULT_SELECTION,
				name: "acme-prod",
				tier: "enterprise",
				runners: { size: "r-160", count: 12 },
				cloud: "byoc",
				region: "aws:us-east-1",
				allowlist: ["10.0.0.0/8", "203.0.113.42/32"],
			}}
			onCreate={(s) => console.log("create", s)}
		/>
	</Frame>
);

export const Dashboard: Story = () => (
	<Frame>
		<DashboardHarness
			initial={{
				...DEFAULT_SELECTION,
				allowlist: ["203.0.113.42/32", "198.51.100.0/24"],
			}}
			deployedDescription={CHAT_DESCRIPTION}
		/>
	</Frame>
);

export const DashboardFreshCluster: Story = () => (
	<Frame>
		<DashboardHarness initial={DEFAULT_SELECTION} />
	</Frame>
);

export const DashboardHetzner: Story = () => (
	<Frame>
		<DashboardHarness
			initial={{
				...DEFAULT_SELECTION,
				name: "arena-eu",
				tier: "pro",
				runners: { size: "r-20", count: 3 },
				cloud: "hetzner",
				region: "fsn1",
				allowlist: ["0.0.0.0/0"],
			}}
			deployedDescription="Multiplayer .io game with 60Hz tick rooms of up to 32 players, matchmaking and a global leaderboard."
		/>
	</Frame>
);

export const FullFlow: Story = () => (
	<Frame>
		<ClusterOnboardingFlow />
	</Frame>
);
