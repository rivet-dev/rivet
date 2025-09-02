import {
	type DisplayUtilitiesProps,
	getDisplayClass,
	omitDisplayProps,
} from "./display";
import {
	getHeightClass,
	type HeightUtilitiesProps,
	omitHeightProps,
} from "./height";
import {
	getMarginClass,
	type MarginUtilitiesProps,
	omitMarginProps,
} from "./margin";
import {
	getMinHeightClass,
	type MinHeightUtilitiesProps,
	omitMinHeightProps,
} from "./min-height";
import {
	getMinWidthClass,
	type MinWidthUtilitiesProps,
	omitMinWidthProps,
} from "./min-width";
import {
	getPaddingClass,
	omitPaddingProps,
	type PaddingUtilitiesProps,
} from "./padding";
import {
	getTextAlignClass,
	omitTextAlignProps,
	type TextAlignUtilitiesProps,
} from "./text-align";
import {
	getWidthClass,
	omitWidthProps,
	type WidthUtilitiesProps,
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
