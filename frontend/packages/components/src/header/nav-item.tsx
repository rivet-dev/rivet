import { Slot } from "@radix-ui/react-slot";
import type { PropsWithChildren } from "react";
import { cn } from "../lib/utils";

interface NavItemProps extends PropsWithChildren<JSX.IntrinsicElements["a"]> {
	asChild?: boolean;
}

export function NavItem({ className, asChild, ...props }: NavItemProps) {
	const Comp = asChild ? Slot : "a";
	return (
		<Comp
			className={cn(
				className,
				"text-muted-foreground data-[active]:text-foreground aria-current-page:text-foreground hover:text-foreground transition-colors",
			)}
			{...props}
		/>
	);
}
