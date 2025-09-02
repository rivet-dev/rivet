import { Slot, Slottable } from "@radix-ui/react-slot";
import { faSpinnerThird, Icon } from "@rivet-gg/icons";
import { cva, type VariantProps } from "class-variance-authority";
import * as React from "react";
import { cn } from "../lib/utils";
import {
	type CommonHelperProps,
	getCommonHelperClass,
	omitCommonHelperProps,
} from "./helpers";

const buttonVariants = cva(
	"group group/button inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:z-10 relative aria-disabled:pointer-events-none aria-disabled:opacity-50 disabled:pointer-events-none disabled:opacity-50",
	{
		variants: {
			variant: {
				default:
					"bg-primary text-primary-foreground hover:bg-primary/90",
				destructive:
					"bg-destructive text-destructive-foreground hover:bg-destructive/90",
				"destructive-outline":
					"border border-destructive/50 bg-transparent hover:bg-destructive hover:text-destructive-foreground",
				outline:
					"border border-input bg-transparent hover:bg-accent hover:text-accent-foreground",
				secondary:
					"border button-secondary bg-secondary text-secondary-foreground hover:bg-secondary/80",
				ghost: "border border-transparent hover:bg-accent/60 hover:text-accent-foreground",
				link: "text-primary underline-offset-4 hover:underline",
			},
			size: {
				default: "h-10 px-4 py-2 gap-1.5 [&_svg]:size-[1em]",
				xs: "h-5 rounded-md px-2 text-xs gap-0.5 [&_svg]:size-2",
				sm: "h-7 rounded-md px-2 text-xs [&_svg]:size-3 gap-1.5",
				lg: "h-11 rounded-md px-8 gap-2 [&_svg]:size-[1em]",
				icon: "h-10 w-10 gap-1.5 [&_svg]:size-[1em]",
				"icon-sm": "h-7 w-7 text-xs [&_svg]:size-3 gap-1.5",
				"icon-xs": "h-5 w-5 text-xs [&_svg]:size-2 gap-0.5",
			},
		},
		defaultVariants: {
			variant: "default",
			size: "default",
		},
	},
);

export interface ButtonProps
	extends VariantProps<typeof buttonVariants>,
		Partial<CommonHelperProps>,
		React.ComponentPropsWithoutRef<"button"> {
	asChild?: boolean;
	isLoading?: boolean;
	startIcon?: React.ReactElement;
	endIcon?: React.ReactElement;
	onClick?: (e?: React.MouseEvent<HTMLButtonElement, MouseEvent>) => void;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
	(
		{
			asChild,
			className,
			variant,
			size,
			startIcon,
			isLoading,
			endIcon,
			disabled,
			children,
			...props
		},
		ref,
	) => {
		const C = asChild ? Slot : "button";

		const isIcon = size?.includes("icon");

		return (
			<C
				className={cn(
					buttonVariants({ variant, size, className }),
					getCommonHelperClass(props),
				)}
				ref={ref}
				{...omitCommonHelperProps(props)}
				disabled={isLoading || disabled}
			>
				{isLoading ? (
					<Icon
						icon={faSpinnerThird}
						className={cn("h-4 w-4 animate-spin")}
					/>
				) : startIcon ? (
					React.cloneElement(startIcon, startIcon.props)
				) : null}
				{isIcon && isLoading ? null : <Slottable>{children}</Slottable>}
				{endIcon ? React.cloneElement(endIcon, endIcon.props) : null}
			</C>
		);
	},
);
Button.displayName = "Button";

export { Button, buttonVariants };
