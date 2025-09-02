import type { Responsive } from "./types";
import { getResponsiveValue } from "./utilities";

export const GRID_COLUMNS_VALUES = [
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
] as const;

type GridColumnsValues = (typeof GRID_COLUMNS_VALUES)[number];

export interface GridColumnsUtilitiesProps {
	columns: Responsive<GridColumnsValues>;
}

export function omitGridColumnsProps<
	T extends Partial<GridColumnsUtilitiesProps>,
>(props: T): Omit<T, keyof GridColumnsUtilitiesProps> {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { columns, ...rest } = props;
	return rest;
}

export function getGridColumnsClass(props: Partial<GridColumnsUtilitiesProps>) {
	const { columns } = props;

	return [columns && getResponsiveValue(columns, "grid-cols")]
		.filter(Boolean)
		.join(" ");
}
