import type { Responsive } from "./types";
import { getResponsiveValue } from "./utilities";

export const MIN_WIDTH_VALUES = [
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
	"60",
] as const;

type MinWidthValues = (typeof MIN_WIDTH_VALUES)[number];

export interface MinWidthUtilitiesProps {
	minW: Responsive<MinWidthValues>;
}

export function omitMinWidthProps<T extends Partial<MinWidthUtilitiesProps>>(
	props: T,
): Omit<T, keyof MinWidthUtilitiesProps> {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { minW, ...rest } = props;
	return rest;
}

export function getMinWidthClass(props: Partial<MinWidthUtilitiesProps>) {
	const { minW } = props;

	return [minW && getResponsiveValue(minW, "min-w")]
		.filter(Boolean)
		.join(" ");
}
