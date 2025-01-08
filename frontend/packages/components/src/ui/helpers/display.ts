import type { Responsive } from "./types";
import { getResponsiveValue } from "./utilities";

export const DISPLAY_VALUES = [
	"hidden",
	"block",
	"flex",
	"table-cell",
] as const;

type DisplayValues = (typeof DISPLAY_VALUES)[number];

export interface DisplayUtilitiesProps {
	display: Responsive<DisplayValues>;
}

export function omitDisplayProps<T extends Partial<DisplayUtilitiesProps>>(
	props: T,
): Omit<T, keyof DisplayUtilitiesProps> {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { display, ...rest } = props;
	return rest;
}

export function getDisplayClass(props: Partial<DisplayUtilitiesProps>) {
	const { display } = props;

	return [display && getResponsiveValue(display, "", { useDash: false })]
		.filter(Boolean)
		.join(" ");
}
