import { projectEnvironmentsQueryOptions } from "@/domains/project/queries";
import {
	Button,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	Code,
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
import { ProjectEnvironmentsTableActions } from "../components/project-environments-table-actions";

interface ProjectEnvironmentsViewProps {
	projectId: string;
	projectNameId: string;
}

export function ProjectEnvironmentsView({
	projectId,
	projectNameId,
}: ProjectEnvironmentsViewProps) {
	const { data } = useSuspenseQuery(
		projectEnvironmentsQueryOptions(projectId),
	);

	const navigate = useNavigate();

	return (
		<Card w="full">
			<CardHeader>
				<Flex items="center" gap="4" justify="between">
					<CardTitle>Environments</CardTitle>
					<Button variant="secondary" size="icon" asChild>
						<Link to="." search={{ modal: "create-environment" }}>
							<Icon icon={faPlus} />
						</Link>
					</Button>
				</Flex>
			</CardHeader>
			<CardContent>
				<Table>
					<TableHeader>
						<TableRow>
							<TableHead w="1/2">Name</TableHead>
							<TableHead w="1/2">Slug</TableHead>
							<TableHead />
						</TableRow>
					</TableHeader>
					<TableBody>
						{data.map((environment) => (
							<TableRow
								key={environment.namespaceId}
								isClickable
								onClick={() => {
									navigate({
										to: "/projects/$projectNameId/environments/$environmentNameId",
										params: {
											projectNameId,
											environmentNameId:
												environment.nameId,
										},
									});
								}}
							>
								<TableCell>
									<Text>{environment.displayName}</Text>
								</TableCell>
								<TableCell>
									<Code>{environment.nameId}</Code>
								</TableCell>
								<TableCell>
									<ProjectEnvironmentsTableActions />
								</TableCell>
							</TableRow>
						))}
					</TableBody>
				</Table>
			</CardContent>
		</Card>
	);
}
