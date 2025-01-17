import { usePathname } from "next/navigation";

import routes from "@/generated/routes.json";

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
