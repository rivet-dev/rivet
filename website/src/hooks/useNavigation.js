import { useState, useEffect } from "react";
import routes from "@/generated/routes.json";

function usePathname() {
	const [pathname, setPathname] = useState("");
	useEffect(() => {
		setPathname(window.location.pathname);
	}, []);
	return pathname;
}

export const useNavigation = () => {
	const pathname = usePathname();
	const page = routes.pages[pathname];
	const tableOfContents = page?.headings ?? null;
	return {
		navigation: {},
		page,
		tableOfContents,
	};
};
