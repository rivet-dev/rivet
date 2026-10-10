import { useState } from "react";
import { ClusterDashboardPage } from "./cluster-dashboard-page";
import { CreateClusterPage } from "./create-cluster-page";
import {
	clampRunners,
	type ClusterSelection,
	DEFAULT_SELECTION,
	type RunnerConfig,
} from "./model";

// Design mock of the new cluster onboarding: an Aiven-style single-page
// create flow with a checkout rail, then the cluster dashboard with the agent
// bar, endpoints + admin token, deployment map, builds, logs, billing and the
// IP allowlist. Nothing here talks to a backend.
//
// Rendered both by the Ladle stories and by the dev-only `/mock/...` route so
// the mock can be reviewed inside the real dashboard shell.

export const CHAT_DESCRIPTION =
	"A team chat app with channels, DMs, typing indicators and presence. Messages need to persist and be searchable.";

// A cluster created this long ago has every node ready, so stories and the
// `?view=dashboard` deep link skip the provisioning animation.
const LONG_AGO_MS = 60 * 60 * 1000;

export function DashboardHarness({
	initial,
	deployedDescription,
	createdAt = Date.now() - LONG_AGO_MS,
}: {
	initial: ClusterSelection;
	deployedDescription?: string;
	createdAt?: number;
}) {
	const [selection, setSelection] = useState(initial);
	// When runner `i` was added relative to `createdAt` (0 = at creation), so
	// the deployment map shows new ones provisioning while the rest stay ready.
	const [runnerAddedMs, setRunnerAddedMs] = useState<number[]>(() =>
		Array.from({ length: initial.runners.count }, () => 0),
	);

	const setRunners = (runners: RunnerConfig) => {
		setSelection((s) => {
			const added = runners.count - s.runners.count;
			if (added > 0) {
				const now = Date.now() - createdAt;
				setRunnerAddedMs((prev) => [
					...prev,
					...Array.from({ length: added }, () => now),
				]);
			} else if (added < 0) {
				setRunnerAddedMs((prev) => prev.slice(0, prev.length + added));
			}
			return { ...s, runners };
		});
	};

	return (
		<ClusterDashboardPage
			selection={selection}
			createdAt={createdAt}
			runnerAddedMs={runnerAddedMs}
			onAllowlistChange={(allowlist) =>
				setSelection((s) => ({ ...s, allowlist }))
			}
			onTierChange={(tier) =>
				setSelection((s) => ({
					...s,
					tier,
					runners: clampRunners(tier, s.runners),
				}))
			}
			onRunnersChange={setRunners}
			deployedDescription={deployedDescription}
		/>
	);
}

/**
 * Full flow: create -> dashboard. The cluster lands on its dashboard
 * immediately and the header and deployment map show nodes provisioning.
 *
 * `initialView="dashboard"` skips straight to a populated, fully provisioned
 * dashboard, handy for linking reviewers at a later part of the mock.
 */
export function ClusterOnboardingFlow({
	initialView = "create",
}: {
	initialView?: "create" | "dashboard";
}) {
	const [created, setCreated] = useState<{
		selection: ClusterSelection;
		createdAt: number;
	} | null>(
		initialView === "dashboard"
			? {
					selection: {
						...DEFAULT_SELECTION,
						allowlist: ["203.0.113.42/32", "198.51.100.0/24"],
					},
					createdAt: Date.now() - LONG_AGO_MS,
				}
			: null,
	);

	if (created) {
		return (
			<DashboardHarness
				initial={created.selection}
				createdAt={created.createdAt}
				deployedDescription={
					initialView === "dashboard" ? CHAT_DESCRIPTION : undefined
				}
			/>
		);
	}
	return (
		<CreateClusterPage
			onCreate={(selection) =>
				setCreated({ selection, createdAt: Date.now() })
			}
		/>
	);
}
