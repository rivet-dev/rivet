import type { Responsive } from "./types";
import { getResponsiveValue } from "./utilities";

export const MARGIN_VALUES = [
	"-4",
	"-2",
	"0",
	"2",
	"4",
	"6",
	"8",
	"10",
] as const;

type MarginValues = (typeof MARGIN_VALUES)[number];

export interface MarginUtilitiesProps {
	m: Responsive<MarginValues>;
	mx: Responsive<MarginValues>;
	my: Responsive<MarginValues>;
	mt: Responsive<MarginValues>;
	mb: Responsive<MarginValues>;
	mr: Responsive<MarginValues>;
	ml: Responsive<MarginValues>;
}

export function omitMarginProps<T extends Partial<MarginUtilitiesProps>>(
	props: T,
): Omit<T, keyof MarginUtilitiesProps> {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { m, mx, my, mt, mb, mr, ml, ...rest } = props;
	return rest;
}

export function getMarginClass(props: Partial<MarginUtilitiesProps>) {
	const { m, mx, my, mt, mb, mr, ml } = props;

	return [
		m && getResponsiveValue(m, "m"),
		mx && getResponsiveValue(mx, "mx"),
		my && getResponsiveValue(my, "my"),
		mt && getResponsiveValue(mt, "mt"),
		mb && getResponsiveValue(mb, "mb"),
		mr && getResponsiveValue(mr, "mr"),
		ml && getResponsiveValue(ml, "ml"),
	]
		.filter(Boolean)
		.join(" ");
}
