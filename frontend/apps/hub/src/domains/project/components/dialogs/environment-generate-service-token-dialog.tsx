import type { DialogContentProps } from "@/hooks/use-dialog";
import {
	Button,
	CopyArea,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Flex,
	Text,
} from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { projectEnvTokenServiceQueryOptions } from "../../queries";

interface ContentProps extends DialogContentProps {
	projectId: string;
	environmentId: string;
}

export default function ProjectEnvGenerateServiceTokenDialogContent({
	projectId,
	environmentId,
	onClose,
}: ContentProps) {
	const { data } = useSuspenseQuery(
		projectEnvTokenServiceQueryOptions({ projectId, environmentId }),
	);
	return (
		<>
			<DialogHeader>
				<DialogTitle>Create Environment Service Token</DialogTitle>
			</DialogHeader>
			<Flex gap="4" direction="col">
				<Text>
					Copy this token to your clipboard. You will not be able to
					access this token again.
				</Text>
				<CopyArea value={data} isConfidential />
			</Flex>
			<DialogFooter>
				<Button onClick={onClose}>Close</Button>
			</DialogFooter>
		</>
	);
}
