import { Icon, type IconProp } from "@rivet-gg/icons";
import { Link, type LinkProps } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { Button, cn } from "@/components";

export interface HeaderLinkProps extends LinkProps {
	icon?: IconProp;
	className?: string;
	children?: ReactNode;
}

export function HeaderLink({
	icon,
	children,
	className,
	...props
}: HeaderLinkProps) {
	return (
		<Button
			asChild
			variant="ghost"
			{...props}
			className={cn(
				"relative",
				"text-muted-foreground px-2 mx-2 aria-current-page:text-foreground relative mb-1 h-auto py-1",
				className,
			)}
			startIcon={
				icon ? <Icon className={cn("size-5")} icon={icon} /> : undefined
			}
		>
			<Link to={props.to}>
				{children}
				<div className="absolute inset-x-0 -bottom-2 z-[1] h-[2px] rounded-sm bg-primary group-data-active:block hidden" />
			</Link>
		</Button>
	);
}
