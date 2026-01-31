import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import * as React from "react";
import { cn } from "../lib/utils";
import {
	type CommonHelperProps,
	getCommonHelperClass,
	omitCommonHelperProps,
} from "./helpers";

const badgeVariants = cva(
	"inline-flex items-center tracking-normal rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 whitespace-nowrap max-w-full overflow-hidden truncate",
	{
		variants: {
			variant: {
				default:
					"border-transparent border-primary bg-primary/10 text-primary",
				secondary:
					"border-transparent bg-secondary text-secondary-foreground ",
				destructive:
					"border-transparent bg-destructive text-destructive-foreground",
				"destructive-muted":
					"border-transparent bg-muted-destructive text-muted-destructive-foreground",
				warning: "border-warning/60 text-foreground",
				outline: "text-foreground",
				premium: "border-transparent",
				"premium-blue": "border-transparent",
			},
		},
		defaultVariants: {
			variant: "default",
		},
	},
);

export interface BadgeProps
	extends React.HTMLAttributes<HTMLDivElement>,
		VariantProps<typeof badgeVariants>,
		Partial<CommonHelperProps> {
	asChild?: boolean;
}

const Badge = React.forwardRef<HTMLDivElement, BadgeProps>(
	({ className, variant, asChild, children, ...props }, ref) => {
		const Comp = asChild ? Slot : "div";

		if (variant === "premium" || variant === "premium-blue") {
			const isBlue = variant === "premium-blue";
			const gradientClasses = isBlue
				? "from-blue-500 via-sky-400 to-blue-500"
				: "from-primary via-orange-400 to-primary";
			const shimmerClasses = isBlue
				? "via-blue-500/10"
				: "via-primary/10";

			return (
				<div
					ref={ref}
					className={cn(
						"relative inline-flex items-center justify-center rounded-full px-2.5 py-0.5 shrink-0",
						`bg-gradient-to-r ${gradientClasses}`,
						getCommonHelperClass(props),
						className,
					)}
					{...omitCommonHelperProps(props)}
				>
					<div className="absolute inset-px rounded-full bg-background" />
					<div className="pointer-events-none absolute inset-px overflow-hidden rounded-full">
						<div
							className={cn(
								`absolute inset-0 bg-gradient-to-r from-transparent to-transparent animate-shimmer-slide`,
								shimmerClasses,
							)}
						/>
					</div>
					<span
						className={cn(
							`relative z-10 text-xs font-semibold bg-gradient-to-r bg-clip-text text-transparent whitespace-nowrap`,
							gradientClasses,
						)}
					>
						{children}
					</span>
				</div>
			);
		}

		return (
			<Comp
				ref={ref}
				className={cn(
					badgeVariants({ variant }),
					getCommonHelperClass(props),
					className,
				)}
				{...omitCommonHelperProps(props)}
			>
				{children}
			</Comp>
		);
	},
);

export { Badge, badgeVariants };
