import { Button, cn } from "@rivet-gg/components";
import { Icon, type IconProp } from "@rivet-gg/icons";
import { Link, type LinkProps, useMatchRoute } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { type ReactNode, useContext } from "react";
import { MobileBreadcrumbsContext } from "../breadcrumbs/mobile-breadcrumbs";

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
	const isMobile = useContext(MobileBreadcrumbsContext);

	const match = useMatchRoute();

	// <Link/> does not support function children (bug?)
	const isCurrent = match({ to: props.to, ...props });

	return (
		<Button
			asChild
			variant="ghost"
			{...props}
			className={cn(
				"relative",
				isMobile &&
					"text-muted-foreground hover:text-foreground text-lg data-active:text-foreground justify-start h-auto",
				!isMobile &&
					"text-muted-foreground px-2 mx-2 aria-current-page:text-foreground relative mb-1 h-auto py-1",
				className,
			)}
			startIcon={
				icon ? (
					<Icon
						className={cn("size-5", isMobile && "size-4")}
						icon={icon}
					/>
				) : undefined
			}
		>
			<Link to={props.to}>
				{children}
				{isCurrent && !isMobile ? (
					<motion.div
						id="header-active-link"
						layoutId="header-active-link"
						className="absolute inset-x-0 -bottom-2 z-[1] h-[2px] rounded-sm bg-primary"
					/>
				) : null}
			</Link>
		</Button>
	);
}
