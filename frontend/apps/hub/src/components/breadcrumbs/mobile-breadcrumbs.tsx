import { createContext } from "react";
import { Breadcrumbs } from "./breadcrumbs";

export const MobileBreadcrumbsContext = createContext(false);

export function MobileBreadcrumbs() {
	return (
		<MobileBreadcrumbsContext.Provider value={true}>
			<Breadcrumbs />
		</MobileBreadcrumbsContext.Provider>
	);
}
