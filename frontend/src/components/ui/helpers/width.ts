import type { Responsive } from "./types";
import { getResponsiveValue } from "./utilities";

export const WIDTH_VALUES = ["1/2", "1/3", "2/3", "16", "full"] as const;

type WidthValues = (typeof WIDTH_VALUES)[number];

export interface WidthUtilitiesProps {
	w: Responsive<WidthValues>;
}

export function omitWidthProps<T extends Partial<WidthUtilitiesProps>>(
	props: T,
): Omit<T, keyof WidthUtilitiesProps> {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { w, ...rest } = props;
	return rest;
}

export function getWidthClass(props: Partial<WidthUtilitiesProps>) {
	const { w } = props;

	return [w && getResponsiveValue(w, "w")].filter(Boolean).join(" ");
}
