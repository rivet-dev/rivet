import type { GroupProjects } from "@/domains/project/queries";
import {
	Button,
	Card,
	CardContent,
	CardHeader,
	Flex,
	Grid,
	LargeText,
	Skeleton,
} from "@rivet-gg/components";
import { Icon, faArrowRight, faPlus } from "@rivet-gg/icons";

import { ProjectTile } from "@/domains/project/components/project-tile";
import { Link } from "@tanstack/react-router";
import { GroupAvatar } from "./group-avatar";
import { GroupEmptyAlert } from "./group-empty-alert";

interface GroupProps extends GroupProjects {}

export function Group(props: GroupProps) {
	const { groupId, displayName, avatarUrl, projects, isDeveloper } = props;

	return (
		<Card my="4">
			<CardHeader>
				<Flex direction="row" justify="between">
					<Flex asChild direction="row" items="center" gap="4">
						<Link to="/teams/$groupId" params={{ groupId }}>
							<GroupAvatar
								displayName={displayName}
								avatarUrl={avatarUrl}
							/>
							<LargeText>{displayName}</LargeText>
						</Link>
					</Flex>
					<Flex>
						{isDeveloper ? (
							<Button asChild variant="ghost" size="icon">
								<Link
									to="/"
									search={{
										modal: "create-project",
										groupId,
									}}
								>
									<Icon icon={faPlus} />
								</Link>
							</Button>
						) : null}
						<Button asChild variant="ghost" size="icon">
							<Link to="/teams/$groupId" params={{ groupId }}>
								<Icon icon={faArrowRight} />
							</Link>
						</Button>
					</Flex>
				</Flex>
			</CardHeader>
			<CardContent>
				{projects.length === 0 ? (
					<GroupEmptyAlert
						groupId={groupId}
						showCreateButton={isDeveloper}
					/>
				) : (
					<Grid columns={{ initial: "1", md: "4" }} gap="4">
						{projects.map((project) => (
							<Link
								key={project.gameId}
								to="/projects/$projectNameId"
								params={{ projectNameId: project.nameId }}
							>
								<ProjectTile {...project} />
							</Link>
						))}
					</Grid>
				)}
			</CardContent>
		</Card>
	);
}

Group.Skeleton = () => {
	return <Skeleton className="my-4 h-64 w-full" />;
};
