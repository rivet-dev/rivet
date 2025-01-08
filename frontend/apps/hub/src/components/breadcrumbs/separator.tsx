import { Icon, faChevronRight } from "@rivet-gg/icons";
import { useContext } from "react";
import { MobileBreadcrumbsContext } from "./mobile-breadcrumbs";

export function Separator() {
	const isMobile = useContext(MobileBreadcrumbsContext);
	if (isMobile) return null;
	return (
		<Icon icon={faChevronRight} className="text-muted-foreground size-4" />
	);
}
