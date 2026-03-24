import {
	createContext,
	type ReactNode,
	useCallback,
	useContext,
	useState,
} from "react";

export const DEFAULT_BRUSH_RANGE_MS = 24 * 60 * 60 * 1000;
export const MAX_BRUSH_RANGE_MS = 7 * 24 * 60 * 60 * 1000;

function defaultBrushDomain(): [Date, Date] {
	const now = new Date();
	return [new Date(now.getTime() - DEFAULT_BRUSH_RANGE_MS), now];
}

function clampBrushDomain(domain: [Date, Date]): [Date, Date] {
	const [start, end] = domain;
	const rangeMs = end.getTime() - start.getTime();
	if (rangeMs <= MAX_BRUSH_RANGE_MS) return domain;
	return [new Date(end.getTime() - MAX_BRUSH_RANGE_MS), end];
}

interface ChartSyncState {
	/** The timestamp (ms epoch) the user is currently hovering over, or null. */
	hoveredTimestamp: number | null;
	/** The brushed sub-range within the full time window. Always set — defaults to last 1d. */
	brushDomain: [Date, Date];
}

interface ChartSyncContextValue extends ChartSyncState {
	setHoveredTimestamp: (ts: number | null) => void;
	setBrushDomain: (domain: [Date, Date] | null) => void;
}

const ChartSyncContext = createContext<ChartSyncContextValue | null>(null);

export function ChartSyncProvider({ children }: { children: ReactNode }) {
	const [hoveredTimestamp, setHoveredTimestamp] = useState<number | null>(
		null,
	);
	const [brushDomain, setBrushDomainState] =
		useState<[Date, Date]>(defaultBrushDomain);

	const handleSetHovered = useCallback((ts: number | null) => {
		setHoveredTimestamp(ts);
	}, []);

	const handleSetBrush = useCallback((domain: [Date, Date] | null) => {
		setBrushDomainState(
			domain ? clampBrushDomain(domain) : defaultBrushDomain(),
		);
	}, []);

	return (
		<ChartSyncContext.Provider
			value={{
				hoveredTimestamp,
				brushDomain,
				setHoveredTimestamp: handleSetHovered,
				setBrushDomain: handleSetBrush,
			}}
		>
			{children}
		</ChartSyncContext.Provider>
	);
}

export function useChartSync(): ChartSyncContextValue {
	const ctx = useContext(ChartSyncContext);
	if (!ctx) {
		throw new Error("useChartSync must be used within a ChartSyncProvider");
	}
	return ctx;
}
