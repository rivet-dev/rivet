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
import { projectTokenCloudQueryOptions } from "../../queries";

interface ContentProps extends DialogContentProps {
	projectId: string;
}

export default function ProjectGenerateCloudTokenDialogContent({
	projectId,
	onClose,
}: ContentProps) {
	const { data } = useSuspenseQuery(
		projectTokenCloudQueryOptions({ projectId }),
	);
	return (
		<>
			<DialogHeader>
				<DialogTitle>Create Project Cloud Token</DialogTitle>
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
