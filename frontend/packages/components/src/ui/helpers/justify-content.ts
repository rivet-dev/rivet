import type { Responsive } from "./types";
import { getResponsiveValue } from "./utilities";

export const JUSTIFY_CONTENT_VALUES = [
	"start",
	"end",
	"center",
	"between",
	"around",
] as const;

type JustifyContentValues = (typeof JUSTIFY_CONTENT_VALUES)[number];

export interface JustifyContentUtilitiesProps {
	justify: Responsive<JustifyContentValues>;
}

export function omitJustifyContentProps<
	T extends Partial<JustifyContentUtilitiesProps>,
>(props: T): Omit<T, keyof JustifyContentUtilitiesProps> {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { justify, ...rest } = props;
	return rest;
}

export function getJustifyContentClass(
	props: Partial<JustifyContentUtilitiesProps>,
) {
	const { justify } = props;

	return [justify && getResponsiveValue(justify, "justify")]
		.filter(Boolean)
		.join(" ");
}
