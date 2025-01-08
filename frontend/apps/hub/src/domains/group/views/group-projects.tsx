import { BillingPlanBadge } from "@/domains/project/components/billing/billing-plan-badge";
import { ProjectTableActions } from "@/domains/project/components/project-table-actions";
import { groupProjectsQueryOptions } from "@/domains/project/queries";
import {
	Button,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	Flex,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
	Text,
} from "@rivet-gg/components";
import { Icon, faPlus } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";

interface GroupProjectsProps {
	groupId: string;
}

export function GroupProjects({ groupId }: GroupProjectsProps) {
	const { data } = useSuspenseQuery(groupProjectsQueryOptions(groupId));

	const navigate = useNavigate();

	return (
		<>
			<Card w="full">
				<CardHeader>
					<Flex items="center" gap="4" justify="between">
						<CardTitle>Projects</CardTitle>
						<Link to="." search={{ modal: "create-group-project" }}>
							<Button variant="secondary" size="sm">
								<Icon icon={faPlus} />
							</Button>
						</Link>
					</Flex>
				</CardHeader>
				<CardContent>
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead>Name</TableHead>
								<TableHead w="16" />
							</TableRow>
						</TableHeader>
						<TableBody>
							{data.projects.length === 0 ? (
								<TableRow>
									<TableCell colSpan={3}>
										<Text textAlign="center">
											There's no projects yet.
										</Text>
									</TableCell>
								</TableRow>
							) : null}
							{data.projects.map((project) => (
								<TableRow
									key={project.gameId}
									isClickable
									onClick={() => {
										navigate({
											to: "/projects/$projectNameId",
											params: {
												projectNameId: project.nameId,
											},
										});
									}}
								>
									<TableCell>
										<Text asChild>
											<Flex items="center" gap="2">
												{project.displayName}{" "}
												<BillingPlanBadge
													projectId={project.gameId}
												/>
											</Flex>
										</Text>
									</TableCell>
									<TableCell>
										<ProjectTableActions />
									</TableCell>
								</TableRow>
							))}
						</TableBody>
					</Table>
				</CardContent>
			</Card>
		</>
	);
}
