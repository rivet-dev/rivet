import { createRivetKit } from "@rivetkit/react";
import { useEffect, useMemo, useState } from "react";
import type {
	UserSessionPreferences,
	UserSessionState,
	registry,
} from "../src/actors.ts";

const { useActor } = createRivetKit<typeof registry>(
	`${window.location.origin}/api/rivet`,
);

const REGION_OPTIONS = [
	{
		id: "us-west",
		label: "US West",
		city: "Oregon",
		latencyHint: "18-32ms",
		map: { x: 165, y: 190 },
	},
	{
		id: "us-east",
		label: "US East",
		city: "Virginia",
		latencyHint: "20-36ms",
		map: { x: 290, y: 185 },
	},
	{
		id: "eu-west",
		label: "EU West",
		city: "Dublin",
		latencyHint: "32-48ms",
		map: { x: 430, y: 160 },
	},
	{
		id: "ap-south",
		label: "AP South",
		city: "Mumbai",
		latencyHint: "45-70ms",
		map: { x: 560, y: 210 },
	},
];

const THEMES: Array<{ id: UserSessionPreferences["theme"]; label: string }> = [
	{ id: "light", label: "Light" },
	{ id: "dark", label: "Dark" },
];

const LANGUAGES: Array<{ id: UserSessionPreferences["language"]; label: string }> = [
	{ id: "en", label: "English" },
	{ id: "es", label: "Espanol" },
	{ id: "fr", label: "Francais" },
];

const PAGE_VISITS = ["Dashboard", "Insights", "Billing", "Settings", "Reports"];

function formatTimestamp(timestamp?: number) {
	if (!timestamp) return "Awaiting login";
	return new Date(timestamp).toLocaleString();
}

function regionLabel(regionId: string) {
	return REGION_OPTIONS.find((region) => region.id === regionId)?.label ?? regionId;
}

function RegionMap({ activeRegion }: { activeRegion: string }) {
	return (
		<svg className="world-map" viewBox="0 0 720 360" role="img">
			<title>Global data locality map</title>
			<g className="map-lands">
				<path d="M68 98l74-24 86 22 24 50-18 54-70 24-64-26-26-44z" />
				<path d="M226 216l34-18 40 20-6 40-36 38-30-22z" />
				<path d="M350 118l62-24 54 14 18 30-24 42-68 12-40-34z" />
				<path d="M394 200l58-16 54 20 18 42-26 52-68 8-40-40z" />
				<path d="M520 124l110-10 46 34-22 48-86 18-66-34z" />
				<path d="M556 232l70-18 52 22-10 44-52 30-56-26z" />
			</g>
			<g className="map-grid">
				<line x1="60" y1="60" x2="660" y2="60" />
				<line x1="60" y1="180" x2="660" y2="180" />
				<line x1="60" y1="300" x2="660" y2="300" />
			</g>
			<g className="map-markers">
				{REGION_OPTIONS.map((region) => {
					const isActive = region.id === activeRegion;
					return (
						<g
							key={region.id}
							className={isActive ? "map-marker active" : "map-marker"}
						>
							<circle
								cx={region.map.x}
								cy={region.map.y}
								r={isActive ? 11 : 9}
							/>
							<text x={region.map.x + 14} y={region.map.y + 4}>
								{region.label}
							</text>
						</g>
					);
				})}
			</g>
		</svg>
	);
}

export function App() {
	const [selectedRegion, setSelectedRegion] = useState(REGION_OPTIONS[1]?.id ?? "us-east");
	const [session, setSession] = useState<UserSessionState | null>(null);
	const [reportedRegion, setReportedRegion] = useState<string>("");
	const [latencyMs, setLatencyMs] = useState<number | null>(null);
	const [userId] = useState(() => crypto.randomUUID());

	const actor = useActor({
		name: "userSession",
		key: ["user", userId],
		createInRegion: selectedRegion,
		createWithInput: { region: selectedRegion },
	});

	const activeRegion = reportedRegion || session?.region || selectedRegion;
	const activeRegionLabel = regionLabel(activeRegion);

	useEffect(() => {
		if (!actor.connection) return;
		const conn = actor.connection;

		let cancelled = false;

		const loadSession = async () => {
			const start = performance.now();
			const [snapshot, region] = await Promise.all([
				conn.getSession(),
				conn.getRegion(),
			]);
			if (cancelled) return;
			setSession(snapshot);
			setReportedRegion(region);
			setLatencyMs(Math.round(performance.now() - start));
		};

		loadSession().catch((err: unknown) => {
			console.error("Failed to load session data:", err);
		});

		return () => {
			cancelled = true;
		};
	}, [actor.connection]);

	const handleRegionChange = (regionId: string) => {
		setSelectedRegion(regionId);
		setSession(null);
		setReportedRegion("");
		setLatencyMs(null);
	};

	const updatePreferences = async (next: Partial<UserSessionPreferences>) => {
		if (!actor.connection) return;
		try {
			const start = performance.now();
			const updated = await actor.connection.updatePreferences(next);
			setSession(updated);
			setLatencyMs(Math.round(performance.now() - start));
		} catch (err: unknown) {
			console.error("Failed to update preferences:", err);
		}
	};

	const logActivity = async (page: string, isLogin = false) => {
		if (!actor.connection) return;
		try {
			const start = performance.now();
			const updated = await actor.connection.logActivity({ page, isLogin });
			setSession(updated);
			setLatencyMs(Math.round(performance.now() - start));
		} catch (err: unknown) {
			console.error("Failed to log activity:", err);
		}
	};

	const regionDetails = useMemo(
		() => REGION_OPTIONS.find((region) => region.id === activeRegion),
		[activeRegion],
	);

	return (
		<div className="app">
			<header className="hero">
				<div className="hero-copy">
					<div className="eyebrow">Geo-Distributed Database</div>
					<h1>Edge sessions that follow your users.</h1>
					<p>
						Each user session lives inside a regional Rivet Actor. Preference
						updates and page visits stay close to the user, reducing round trip
						latency while keeping data persistent.
					</p>
					<div className="status-row">
						<span
							className={
								actor.connection ? "status connected" : "status connecting"
							}
						>
							{actor.connection ? "Connected" : "Connecting"}
						</span>
						<span className="status detail">
							Edge latency {latencyMs ? `${latencyMs}ms` : "measuring"}
						</span>
					</div>
				</div>
				<div className="hero-region">
					<div className="region-label">Session stored in</div>
					<div className="region-name">{activeRegionLabel}</div>
					<div className="region-meta">
						<div>Region code: {activeRegion}</div>
						<div>
							Last login: {formatTimestamp(session?.lastLoginAt)}
						</div>
					</div>
				</div>
			</header>

			<section className="grid">
				<div className="panel map-panel">
					<div className="panel-header">
						<div>
							<h2>Data locality map</h2>
							<p>
								Your session state is pinned to a regional data plane.
							</p>
						</div>
						<div className="chip">{activeRegionLabel}</div>
					</div>
					<RegionMap activeRegion={activeRegion} />
					<div className="map-meta">
						<div className="map-stat">
							<span>Nearest edge</span>
							<strong>{regionDetails?.city ?? "Local"}</strong>
						</div>
						<div className="map-stat">
							<span>Estimated RTT</span>
							<strong>{regionDetails?.latencyHint ?? "-"}</strong>
						</div>
						<div className="map-stat">
							<span>Storage mode</span>
							<strong>Edge persistent state</strong>
						</div>
					</div>
				</div>

				<div className="panel session-panel">
					<div className="panel-header">
						<div>
							<h2>Session preferences</h2>
							<p>Update state with actions and see it persist instantly.</p>
						</div>
					</div>
					<div className="form-row">
						<label htmlFor="theme">Theme</label>
						<select
							id="theme"
							value={session?.preferences.theme ?? "light"}
							onChange={(event) =>
								updatePreferences({ theme: event.target.value as "light" | "dark" })
							}
						>
							{THEMES.map((theme) => (
								<option key={theme.id} value={theme.id}>
									{theme.label}
								</option>
							))}
						</select>
					</div>
					<div className="form-row">
						<label htmlFor="language">Language</label>
						<select
							id="language"
							value={session?.preferences.language ?? "en"}
							onChange={(event) =>
								updatePreferences({
									language: event.target.value as "en" | "es" | "fr",
								})
							}
						>
							{LANGUAGES.map((language) => (
								<option key={language.id} value={language.id}>
									{language.label}
								</option>
							))}
						</select>
					</div>
					<div className="form-row">
						<label>Region selection</label>
						<div className="region-buttons">
							{REGION_OPTIONS.map((region) => (
								<button
									key={region.id}
									type="button"
									className={
										region.id === selectedRegion
											? "tag active"
											: "tag"
									}
									onClick={() => handleRegionChange(region.id)}
								>
									{region.label}
								</button>
							))}
						</div>
					</div>
					<div className="divider" />
					<div className="activity-list">
						<div className="activity-header">
							<h3>Recent page visits</h3>
							<button
								type="button"
								className="ghost"
								onClick={() => logActivity("Login", true)}
							>
								Log login
							</button>
						</div>
						{session?.recentActivity.length ? (
							<ul>
								{session.recentActivity.map((entry) => (
									<li key={`${entry.page}-${entry.timestamp}`}>
										<span>{entry.page}</span>
										<time>{formatTimestamp(entry.timestamp)}</time>
									</li>
								))}
							</ul>
						) : (
							<p className="empty">No activity logged yet.</p>
						)}
					</div>
				</div>
			</section>

			<section className="panel activity-panel">
				<div className="panel-header">
					<div>
						<h2>Simulate nearby activity</h2>
						<p>
							Click a page to store activity at the edge and measure latency.
						</p>
					</div>
				</div>
				<div className="page-buttons">
					{PAGE_VISITS.map((page) => (
						<button
							key={page}
							type="button"
							className="page"
							onClick={() => logActivity(page)}
						>
							{page}
						</button>
					))}
				</div>
			</section>
		</div>
	);
}
