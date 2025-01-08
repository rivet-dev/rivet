import { Slot } from "@radix-ui/react-slot";
import { cn } from "@rivet-gg/components";
import { type PropsWithChildren, forwardRef } from "react";

interface NavItemProps extends PropsWithChildren<JSX.IntrinsicElements["a"]> {
	asChild?: boolean;
}

export const NavItem = forwardRef<HTMLAnchorElement, NavItemProps>(
	({ className, asChild, ...props }, ref) => {
		const Comp = asChild ? Slot : "a";
		return (
			<Comp
				ref={ref}
				className={cn(
					"text-muted-foreground hover:text-foreground transition-colors",
					className,
				)}
				{...props}
			/>
		);
	},
);
