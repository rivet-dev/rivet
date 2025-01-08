import {
	Flex,
	SidebarNavigation,
	SidebarPageContent,
	Skeleton,
	cn,
} from "@rivet-gg/components";
import { Icon, faSpinnerThird } from "@rivet-gg/icons";
import { Link, type LinkOptions } from "@tanstack/react-router";
import type { ComponentProps, PropsWithChildren, ReactNode } from "react";
import { BackendEnvironmentDatabaseLink } from "../components/backend/backend-environment-database-link";

const LINKS = [
	{
		to: "/projects/$projectNameId/environments/$environmentNameId/backend",
		text: "Overview",
		exact: true,
	},
	{
		to: "/projects/$projectNameId/environments/$environmentNameId/backend/logs",
		text: "Logs",
	},
	{
		to: "/projects/$projectNameId/environments/$environmentNameId/backend/variables",
		text: "Variables",
	},
] satisfies ({ text: string; exact?: boolean } & LinkOptions)[];

const DatabaseLink = ({
	isLoading,
	onClick,
}: ComponentProps<"button"> & { isLoading?: boolean }) => {
	return (
		<button
			onClick={onClick}
			type="button"
			className="text-left inline-block data-active:text-foreground data-active:font-semibold"
		>
			{isLoading ? (
				<Icon
					icon={faSpinnerThird}
					className={cn("h-4 w-4 animate-spin mr-2")}
				/>
			) : null}
			Database
		</button>
	);
};

interface BackendPageProps {
	projectId: string;
	projectNameId: string;
	environmentId: string;
	environmentNameId: string;
	children: ReactNode;
}

function BackendPage({
	environmentNameId,
	environmentId,
	projectId,
	projectNameId,
	children,
}: BackendPageProps) {
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
					<BackendEnvironmentDatabaseLink
						asChild
						projectId={projectId}
						environmentId={environmentId}
					>
						<DatabaseLink />
					</BackendEnvironmentDatabaseLink>
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

BackendPage.Skeleton = function BackendPageSkeleton() {
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

export { BackendPage as Root, Content };
