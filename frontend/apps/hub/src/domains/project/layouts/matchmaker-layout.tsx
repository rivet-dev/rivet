import {
	Flex,
	SidebarNavigation,
	SidebarPageContent,
	Skeleton,
} from "@rivet-gg/components";
import { Link, type LinkOptions } from "@tanstack/react-router";
import type { PropsWithChildren, ReactNode } from "react";

const LINKS = [
	{
		to: "/projects/$projectNameId/environments/$environmentNameId/lobbies",
		text: "Lobbies",
		exact: true,
	},
	{
		to: "/projects/$projectNameId/environments/$environmentNameId/lobbies/logs",
		text: "Logs",
	},
	{
		to: "/projects/$projectNameId/environments/$environmentNameId/lobbies/settings",
		text: "Settings",
	},
] satisfies ({ text: string; exact?: boolean } & LinkOptions)[];

interface MatchmakerPageProps {
	projectNameId: string;
	environmentNameId: string;
	children: ReactNode;
}

function MatchmakerPage({
	projectNameId,
	environmentNameId,
	children,
}: MatchmakerPageProps) {
	return (
		<SidebarPageContent
			sidebar={
				<SidebarNavigation>
					{LINKS.map((link) => (
						<Link
							key={link.to}
							to={link.to}
							activeOptions={{
								exact: link.exact,
								includeSearch: false,
							}}
							params={{
								projectNameId,
								environmentNameId,
							}}
							className="data-active:text-foreground data-active:font-semibold"
						>
							{link.text}
						</Link>
					))}
				</SidebarNavigation>
			}
		>
			<Flex
				gap="4"
				direction="col"
				className="w-full min-h-0 h-full md:h-auto"
			>
				{children}
			</Flex>
		</SidebarPageContent>
	);
}

MatchmakerPage.Skeleton = function MatchmakerPageSkeleton() {
	return (
		<SidebarPageContent
			sidebar={
				<SidebarNavigation>
					<Skeleton className="w-full h-5" />
					<Skeleton className="w-full h-5" />
					<Skeleton className="w-full h-5" />
					<Skeleton className="w-full h-5" />
				</SidebarNavigation>
			}
		>
			<Flex
				gap="4"
				direction="col"
				className="w-full min-h-0 h-full md:h-auto"
			>
				<Skeleton className="w-full h-56" />
				<Skeleton className="w-full h-56" />
			</Flex>
		</SidebarPageContent>
	);
};

function Content({ children }: PropsWithChildren) {
	return children;
}

Content.Skeleton = function ContentSkeleton() {
	return (
		<>
			<Skeleton className="w-full h-56" />
			<Skeleton className="w-full h-56" />
		</>
	);
};

export { MatchmakerPage as Root, Content };
