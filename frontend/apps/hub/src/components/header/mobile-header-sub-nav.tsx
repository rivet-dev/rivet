import { MobileBreadcrumbsContext } from "../breadcrumbs/mobile-breadcrumbs";
import { HeaderSubNav } from "./header-sub-nav";

export function MobileHeaderSubNav() {
	return (
		<MobileBreadcrumbsContext.Provider value={true}>
			<HeaderSubNav />
		</MobileBreadcrumbsContext.Provider>
	);
}
