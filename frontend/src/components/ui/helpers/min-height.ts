import type { Responsive } from "./types";
import { getResponsiveValue } from "./utilities";

export const MIN_HEIGHT_VALUES = [
	"0",
	"1",
	"2",
	"3",
	"4",
	"5",
	"6",
	"7",
	"8",
	"9",
	"10",
] as const;

type MinHeightValues = (typeof MIN_HEIGHT_VALUES)[number];

export interface MinHeightUtilitiesProps {
	minH: Responsive<MinHeightValues>;
}

export function omitMinHeightProps<T extends Partial<MinHeightUtilitiesProps>>(
	props: T,
): Omit<T, keyof MinHeightUtilitiesProps> {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { minH, ...rest } = props;
	return rest;
}

export function getMinHeightClass(props: Partial<MinHeightUtilitiesProps>) {
	const { minH } = props;

	return [minH && getResponsiveValue(minH, "min-h")]
		.filter(Boolean)
		.join(" ");
}
