import { Button, type ButtonProps, cn } from "@rivet-gg/components";
import { Icon, type IconProp } from "@rivet-gg/icons";
import { type ReactNode, useContext } from "react";
import { MobileBreadcrumbsContext } from "../breadcrumbs/mobile-breadcrumbs";

export interface HeaderLinkProps extends ButtonProps {
	icon?: IconProp;
	children?: ReactNode;
}

export function HeaderLink({
	icon,
	children,
	startIcon,
	...props
}: HeaderLinkProps) {
	const isMobile = useContext(MobileBreadcrumbsContext);

	return (
		<Button
			asChild
			variant="ghost"
			{...props}
			className={cn(
				isMobile &&
					"text-muted-foreground hover:text-foreground text-lg data-active:text-foreground justify-start p-0 h-auto",
				!isMobile &&
					"text-muted-foreground px-2 mx-2 aria-current-page:text-foreground after:content-[' '] aria-current-page:after:bg-primary relative mb-1 h-auto py-1 after:absolute after:inset-x-0 after:-bottom-2 after:z-[-1] after:h-[2px] after:rounded-sm",
				props.className,
			)}
			startIcon={
				startIcon ||
				(icon ? (
					<Icon
						className={cn("size-5", isMobile && "size-4")}
						icon={icon}
					/>
				) : undefined)
			}
		>
			{children}
		</Button>
	);
}
