import type { Responsive } from "./types";
import { getResponsiveValue } from "./utilities";

export const HEIGHT_VALUES = ["1/2", "1/3", "2/3", "16", "full"] as const;

type HeightValues = (typeof HEIGHT_VALUES)[number];

export interface HeightUtilitiesProps {
	h: Responsive<HeightValues>;
}

export function omitHeightProps<T extends Partial<HeightUtilitiesProps>>(
	props: T,
): Omit<T, keyof HeightUtilitiesProps> {
	const { h, ...rest } = props;
	return rest;
}

export function getHeightClass(props: Partial<HeightUtilitiesProps>) {
	const { h } = props;

	return [h && getResponsiveValue(h, "h")].filter(Boolean).join(" ");
}
