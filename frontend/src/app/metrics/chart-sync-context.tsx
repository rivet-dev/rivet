import {
	createContext,
	useCallback,
	useContext,
	useState,
	type ReactNode,
} from "react";

interface ChartSyncState {
	/** The timestamp (ms epoch) the user is currently hovering over, or null. */
	hoveredTimestamp: number | null;
	/** The brushed sub-range within the full time window, or null if no brush is active. */
	brushDomain: [Date, Date] | null;
}

interface ChartSyncContextValue extends ChartSyncState {
	setHoveredTimestamp: (ts: number | null) => void;
	setBrushDomain: (domain: [Date, Date] | null) => void;
}

const ChartSyncContext = createContext<ChartSyncContextValue | null>(null);

export function ChartSyncProvider({ children }: { children: ReactNode }) {
	const [hoveredTimestamp, setHoveredTimestamp] = useState<number | null>(null);
	const [brushDomain, setBrushDomain] = useState<[Date, Date] | null>(null);

	const handleSetHovered = useCallback((ts: number | null) => {
		setHoveredTimestamp(ts);
	}, []);

	const handleSetBrush = useCallback((domain: [Date, Date] | null) => {
		setBrushDomain(domain);
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
