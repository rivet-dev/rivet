import type { Responsive } from "./types";
import { getResponsiveValue } from "./utilities";

export const PADDING_VALUES = ["0", "2", "4", "6", "8", "10"] as const;

type PaddingValues = (typeof PADDING_VALUES)[number];

export interface PaddingUtilitiesProps {
	p: Responsive<PaddingValues>;
	px: Responsive<PaddingValues>;
	py: Responsive<PaddingValues>;
	pt: Responsive<PaddingValues>;
	pb: Responsive<PaddingValues>;
	pr: Responsive<PaddingValues>;
	pl: Responsive<PaddingValues>;
}

export function omitPaddingProps<T extends Partial<PaddingUtilitiesProps>>(
	props: T,
): Omit<T, keyof PaddingUtilitiesProps> {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { p, px, py, pt, pb, pr, pl, ...rest } = props;
	return rest;
}

export function getPaddingClass(props: Partial<PaddingUtilitiesProps>) {
	const { p, px, py, pt, pb, pr, pl } = props;

	return [
		p && getResponsiveValue(p, "p"),
		px && getResponsiveValue(px, "px"),
		py && getResponsiveValue(py, "py"),
		pt && getResponsiveValue(pt, "pt"),
		pb && getResponsiveValue(pb, "pb"),
		pr && getResponsiveValue(pr, "pr"),
		pl && getResponsiveValue(pl, "pl"),
	]
		.filter(Boolean)
		.join(" ");
}
