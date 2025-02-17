import { groupProjectsQueryOptions } from "@/domains/project/queries";
import { faGear, faHome, faUsers } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { HeaderLink } from "../header-link";

interface HeaderLinksLinkProps {
	groupId: string;
}

export function HeaderGroupLinks({ groupId }: HeaderLinksLinkProps) {
	useSuspenseQuery(groupProjectsQueryOptions(groupId));
	return (
		<>
			<HeaderLink
				icon={faHome}
				to="/teams/$groupId"
				activeOptions={{ exact: true }}
				params={{ groupId }}
			>
				Overview
			</HeaderLink>
			<HeaderLink
				icon={faUsers}
				to="/teams/$groupId/members"
				params={{ groupId }}
			>
				Members
			</HeaderLink>
			<HeaderLink
				icon={faGear}
				to="/teams/$groupId/settings"
				params={{ groupId }}
			>
				Settings
			</HeaderLink>
		</>
	);
}
