import { groupProjectsQueryOptions } from "@/domains/project/queries";
import { faGear, faHome, faUsers } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { HeaderLink } from "../header-link";

interface HeaderLinksLinkProps {
	groupId: string;
}

export function HeaderGroupLinks({ groupId }: HeaderLinksLinkProps) {
	useSuspenseQuery(groupProjectsQueryOptions(groupId));
	return (
		<>
			<HeaderLink icon={faHome}>
				<Link
					to="/teams/$groupId"
					activeOptions={{ exact: true }}
					params={{ groupId }}
				>
					Overview
				</Link>
			</HeaderLink>
			<HeaderLink icon={faUsers}>
				<Link to="/teams/$groupId/members" params={{ groupId }}>
					Members
				</Link>
			</HeaderLink>
			<HeaderLink icon={faGear}>
				<Link to="/teams/$groupId/settings" params={{ groupId }}>
					Settings
				</Link>
			</HeaderLink>
		</>
	);
}
