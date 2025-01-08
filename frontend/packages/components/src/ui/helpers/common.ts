import {
	type DisplayUtilitiesProps,
	getDisplayClass,
	omitDisplayProps,
} from "./display";
import {
	type HeightUtilitiesProps,
	getHeightClass,
	omitHeightProps,
} from "./height";
import {
	type MarginUtilitiesProps,
	getMarginClass,
	omitMarginProps,
} from "./margin";
import {
	type MinHeightUtilitiesProps,
	getMinHeightClass,
	omitMinHeightProps,
} from "./min-height";
import {
	type MinWidthUtilitiesProps,
	getMinWidthClass,
	omitMinWidthProps,
} from "./min-width";
import {
	type PaddingUtilitiesProps,
	getPaddingClass,
	omitPaddingProps,
} from "./padding";
import {
	type TextAlignUtilitiesProps,
	getTextAlignClass,
	omitTextAlignProps,
} from "./text-align";
import {
	type WidthUtilitiesProps,
	getWidthClass,
	omitWidthProps,
} from "./width";

export interface CommonHelperProps
	extends MarginUtilitiesProps,
		PaddingUtilitiesProps,
		WidthUtilitiesProps,
		HeightUtilitiesProps,
		MinHeightUtilitiesProps,
		MinWidthUtilitiesProps,
		TextAlignUtilitiesProps,
		DisplayUtilitiesProps {}

export function omitCommonHelperProps(props: Partial<CommonHelperProps>) {
	return omitHeightProps(
		omitDisplayProps(
			omitTextAlignProps(
				omitMinWidthProps(
					omitMinHeightProps(
						omitMarginProps(
							omitPaddingProps(omitWidthProps(props)),
						),
					),
				),
			),
		),
	);
}

export function getCommonHelperClass(props: Partial<CommonHelperProps>) {
	return [
		getMarginClass(props),
		getPaddingClass(props),
		getWidthClass(props),
		getMinHeightClass(props),
		getMinWidthClass(props),
		getTextAlignClass(props),
		getDisplayClass(props),
		getHeightClass(props),
	].join(" ");
}
