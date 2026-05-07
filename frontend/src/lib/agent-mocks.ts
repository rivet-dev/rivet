// Dev-only HTTP mocking surface for agent-driven testing.
//
// Activated when the dev build sees `?mock=1` on the URL. Once active, exposes:
//   window.__rivetMock(pattern, { status, body, method? })
//   window.__rivetClearMocks()
//
// `pattern` is an MSW path matcher (e.g. "*/actors/:id/kv/keys/*"). Mocks are
// persisted to sessionStorage so they survive page reloads, which is the
// common agent-test workflow (set mock, reload to retrigger queries).

import type { HttpHandler } from "msw";

type MockMethod = "get" | "post" | "put" | "delete" | "patch";

type MockSpec = {
	status: number;
	body?: unknown;
	method?: MockMethod;
};

type StoredMock = MockSpec & { pattern: string };

const STORAGE_KEY = "__rivetAgentMocks";

declare global {
	interface Window {
		__rivetMock?: (pattern: string, spec: MockSpec) => void;
		__rivetClearMocks?: () => void;
	}
}

function readStoredMocks(): StoredMock[] {
	try {
		const raw = sessionStorage.getItem(STORAGE_KEY);
		if (!raw) return [];
		const parsed = JSON.parse(raw);
		return Array.isArray(parsed) ? parsed : [];
	} catch {
		return [];
	}
}

function writeStoredMocks(mocks: StoredMock[]) {
	sessionStorage.setItem(STORAGE_KEY, JSON.stringify(mocks));
}

export async function maybeStartAgentMocks() {
	if (!import.meta.env.DEV) return;
	const params = new URLSearchParams(window.location.search);
	if (params.get("mock") !== "1") return;

	const { setupWorker } = await import("msw/browser");
	const { http, HttpResponse } = await import("msw");

	const buildHandler = (m: StoredMock): HttpHandler => {
		const method = m.method ?? "get";
		return http[method](m.pattern, () =>
			HttpResponse.json(m.body ?? null, { status: m.status }),
		);
	};

	const initial = readStoredMocks();
	const worker = setupWorker(...initial.map(buildHandler));
	await worker.start({
		onUnhandledRequest: "bypass",
		quiet: true,
	});

	window.__rivetMock = (pattern, spec) => {
		const stored: StoredMock = { pattern, ...spec };
		const next = [
			...readStoredMocks().filter(
				(m) =>
					m.pattern !== pattern ||
					(m.method ?? "get") !== (spec.method ?? "get"),
			),
			stored,
		];
		writeStoredMocks(next);
		worker.use(buildHandler(stored));
	};

	window.__rivetClearMocks = () => {
		writeStoredMocks([]);
		worker.resetHandlers();
	};

	// eslint-disable-next-line no-console
	console.info(
		`[agent-mocks] active (${initial.length} restored). Use window.__rivetMock(pattern, { status, body }).`,
	);
}
