import type { Responsive } from "./types";
import { getResponsiveValue } from "./utilities";

export const TEXT_ALIGN_VALUES = ["left", "right", "center"] as const;

type TextAlignValues = (typeof TEXT_ALIGN_VALUES)[number];

export interface TextAlignUtilitiesProps {
	textAlign: Responsive<TextAlignValues>;
}

export function omitTextAlignProps<T extends Partial<TextAlignUtilitiesProps>>(
	props: T,
): Omit<T, keyof TextAlignUtilitiesProps> {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { textAlign, ...rest } = props;
	return rest;
}

export function getTextAlignClass(props: Partial<TextAlignUtilitiesProps>) {
	const { textAlign } = props;

	return [textAlign && getResponsiveValue(textAlign, "text")]
		.filter(Boolean)
		.join(" ");
}
