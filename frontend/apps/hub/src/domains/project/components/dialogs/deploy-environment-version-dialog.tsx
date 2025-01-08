import type { DialogContentProps } from "@/hooks/use-dialog";
import {
	Button,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Flex,
	Strong,
	Text,
} from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import {
	projectQueryOptions,
	useUpdateProjectEnvironmentVersionMutation,
} from "../../queries";

interface ContentProps extends DialogContentProps {
	projectId: string;
	environmentId: string;
	versionId: string;
}

export default function DeployEnvironmentVersionDialogContent({
	projectId,
	environmentId,
	versionId,
	onClose,
}: ContentProps) {
	const { data: project } = useSuspenseQuery(projectQueryOptions(projectId));
	const { mutate, isPending } = useUpdateProjectEnvironmentVersionMutation({
		onSuccess: onClose,
	});

	const chosenVersion = project.versions.find(
		(v) => v.versionId === versionId,
	);
	const chosenEnvironment = project.namespaces.find(
		(ns) => ns.namespaceId === environmentId,
	);
	return (
		<>
			<DialogHeader>
				<DialogTitle>
					Deploy version {chosenVersion?.displayName} to{" "}
					{chosenEnvironment?.displayName} of {project.displayName}
				</DialogTitle>
			</DialogHeader>
			<Flex gap="4" direction="col">
				<Text>
					Are you sure you want to deploy version{" "}
					<Strong>{chosenVersion?.displayName}</Strong> created at{" "}
					{chosenVersion?.createTs.toLocaleString()} to environment{" "}
					<Strong>{chosenEnvironment?.displayName}</Strong> of project{" "}
					<Strong>{project.displayName}</Strong>?
				</Text>
			</Flex>
			<DialogFooter>
				<Button
					onClick={() =>
						mutate({ versionId, environmentId, projectId })
					}
					isLoading={isPending}
				>
					Deploy
				</Button>
			</DialogFooter>
		</>
	);
}
