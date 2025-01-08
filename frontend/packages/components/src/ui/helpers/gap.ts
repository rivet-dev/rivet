import type { Responsive } from "./types";
import { getResponsiveValue } from "./utilities";

export const GAP_VALUES = ["0", "1", "2", "4", "6", "8", "10"] as const;

type GapValues = (typeof GAP_VALUES)[number];

export interface GapUtilitiesProps {
	gap: Responsive<GapValues>;
}

export function omitGapProps<T extends Partial<GapUtilitiesProps>>(
	props: T,
): Omit<T, keyof GapUtilitiesProps> {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { gap, ...rest } = props;
	return rest;
}

export function getGapClass(props: Partial<GapUtilitiesProps>) {
	const { gap } = props;

	return [gap && getResponsiveValue(gap, "gap")].filter(Boolean).join(" ");
}
