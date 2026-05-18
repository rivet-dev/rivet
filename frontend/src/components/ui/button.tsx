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
	"group group/button inline-flex items-center justify-center whitespace-nowrap rounded-lg text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:z-10 relative aria-disabled:pointer-events-none aria-disabled:opacity-50 disabled:pointer-events-none disabled:opacity-50",
	{
		variants: {
			variant: {
				default:
					"bg-foreground text-background hover:bg-foreground/90",
				destructive:
					"bg-destructive text-destructive-foreground hover:bg-destructive/85",
				"destructive-outline":
					"border border-destructive/40 bg-transparent text-destructive hover:bg-destructive hover:text-destructive-foreground hover:border-destructive",
				outline:
					"border border-foreground/10 bg-foreground/[0.02] hover:bg-foreground/[0.06] hover:border-foreground/20",
				secondary:
					"border border-foreground/10 bg-card hover:bg-foreground/[0.06] hover:border-foreground/20",
				ghost: "border border-transparent text-muted-foreground hover:bg-foreground/[0.06] hover:text-foreground",
				link: "text-primary underline-offset-4 hover:underline",
			},
			size: {
				default: "h-10 px-4 py-2 gap-1.5 [&_svg]:size-[1em]",
				xs: "h-5 rounded-md px-2 text-xs gap-0.5 [&_svg]:size-2",
				sm: "h-7 rounded-md px-2 text-sm [&_svg]:size-3.5 gap-1.5",
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
	startIcon?: React.ReactElement<any>;
	endIcon?: React.ReactElement<any>;
	onClick?: (e: React.MouseEvent<HTMLButtonElement, MouseEvent>) => void;
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
