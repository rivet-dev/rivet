import { Slot } from "@radix-ui/react-slot";
import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { cn } from "../lib/utils";
import {
	type CommonHelperProps,
	getCommonHelperClass,
	omitCommonHelperProps,
} from "./helpers";
import {
	type AlignItemsValuesUtilitiesProps,
	getAlignItemsClass,
	omitAlignItemsProps,
} from "./helpers/align-items";
import {
	type FlexDirectionUtilitiesProps,
	getFlexDirectionClass,
	omitFlexDirectionProps,
} from "./helpers/flex-direction";
import {
	type GapUtilitiesProps,
	getGapClass,
	omitGapProps,
} from "./helpers/gap";
import {
	getJustifyContentClass,
	type JustifyContentUtilitiesProps,
	omitJustifyContentProps,
} from "./helpers/justify-content";

export interface StackProps
	extends HTMLAttributes<HTMLDivElement>,
		Partial<CommonHelperProps>,
		Partial<GapUtilitiesProps>,
		Partial<FlexDirectionUtilitiesProps>,
		Partial<JustifyContentUtilitiesProps>,
		Partial<AlignItemsValuesUtilitiesProps> {
	children: ReactNode;
	asChild?: boolean;
}

const VStack = (props: Omit<StackProps, "direction">) => {
	return <Flex {...props} direction="col" />;
};

const HStack = (props: Omit<StackProps, "direction">) => {
	return <Flex {...props} direction="row" />;
};

const Flex = forwardRef<HTMLDivElement, StackProps>(
	({ children, className, asChild, ...props }, ref) => {
		const htmlProps = omitAlignItemsProps(
			omitJustifyContentProps(
				omitFlexDirectionProps(
					omitGapProps(omitCommonHelperProps(props)),
				),
			),
		);

		const C = asChild ? Slot : "div";
		return (
			<C
				ref={ref}
				className={cn(
					"flex",
					getCommonHelperClass(props),
					getGapClass(props),
					getFlexDirectionClass(props),
					getJustifyContentClass(props),
					getAlignItemsClass(props),
					className,
				)}
				{...htmlProps}
			>
				{children}
			</C>
		);
	},
);

export { HStack, VStack, Flex };
