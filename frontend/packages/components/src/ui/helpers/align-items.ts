import type { Responsive } from "./types";
import { getResponsiveValue } from "./utilities";

export const ALIGN_ITEMS_VALUES = [
	"start",
	"end",
	"center",
	"baseline",
	"stretch",
] as const;

type AlignItemsValues = (typeof ALIGN_ITEMS_VALUES)[number];

export interface AlignItemsValuesUtilitiesProps {
	items: Responsive<AlignItemsValues>;
}

export function omitAlignItemsProps<
	T extends Partial<AlignItemsValuesUtilitiesProps>,
>(props: T): Omit<T, keyof AlignItemsValuesUtilitiesProps> {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { items, ...rest } = props;
	return rest;
}

export function getAlignItemsClass(
	props: Partial<AlignItemsValuesUtilitiesProps>,
) {
	const { items } = props;

	return [items && getResponsiveValue(items, "items")]
		.filter(Boolean)
		.join(" ");
}
