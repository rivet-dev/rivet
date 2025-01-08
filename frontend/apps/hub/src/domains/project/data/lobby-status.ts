import type { Rivet } from "@rivet-gg/api";

export type LobbyStatus =
	| "running"
	| "not-started"
	| "failed"
	| "closed"
	| "idle"
	| "outdated"
	| "unknown";

export function formatLobbyStatus(status: LobbyStatus) {
	const map: Record<LobbyStatus, string> = {
		running: "Running",
		"not-started": "Not started",
		failed: "Failed",
		closed: "Closed",
		unknown: "Unknown status",
		idle: "Idle",
		outdated: "Outdated",
	};

	return map[status];
}

export function getLiveLobbyStatus(
	lobby: Rivet.cloud.LobbySummaryAnalytics,
): LobbyStatus {
	if (lobby.isClosed) {
		return "closed";
	}
	if (lobby.isIdle) {
		return "idle";
	}
	if (lobby.isOutdated) {
		return "outdated";
	}
	if (lobby.isReady) {
		return "running";
	}
	return "unknown";
}

export function getLobbyStatus(
	status: Rivet.cloud.LogsLobbyStatus,
	startTs: Date | undefined,
): LobbyStatus {
	if (status.stopped?.stopTs) {
		return status.stopped.failed ? "failed" : "closed";
	}

	if (status.running !== undefined) {
		return startTs ? "running" : "not-started";
	}

	return "unknown";
}
