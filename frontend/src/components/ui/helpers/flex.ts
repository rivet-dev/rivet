import type { Responsive } from "./types";
import { getResponsiveValue } from "./utilities";

export const FLEX_VALUES = ["1", "2", "3"] as const;

type FlexValues = (typeof FLEX_VALUES)[number];

export interface FlexUtilitiesProps {
	flex: Responsive<FlexValues>;
}

export function omitFlexProps<T extends Partial<FlexUtilitiesProps>>(
	props: T,
): Omit<T, keyof FlexUtilitiesProps> {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { flex, ...rest } = props;
	return rest;
}

export function getFlexClass(props: Partial<FlexUtilitiesProps>) {
	const { flex } = props;

	return [flex && getResponsiveValue(flex, "flex")].filter(Boolean).join(" ");
}
