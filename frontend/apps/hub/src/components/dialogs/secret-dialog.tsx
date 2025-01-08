import type { DialogContentProps } from "@/hooks/use-dialog";
import { publicUrl } from "@/lib/utils";
import {
	Code,
	DialogHeader,
	DialogTitle,
	Flex,
	Link,
	Text,
} from "@rivet-gg/components";

interface ContentProps extends DialogContentProps {}

export default function SecretDialogContent(props: ContentProps) {
	return (
		<>
			<DialogHeader>
				<DialogTitle>Secret</DialogTitle>
			</DialogHeader>
			<Flex gap="4" direction="col" textAlign="center">
				<img
					className="w-20 mx-auto"
					src={publicUrl("/greg.svg")}
					alt="Mysterious Rivet Frog"
				/>
				<Text>
					Rivet 2.0
					<br />
					<Code>{__APP_GIT_BRANCH__}</Code>@
					<Code>{__APP_GIT_COMMIT__}</Code>
					<br />
					<Code>{__APP_RIVET_NAMESPACE__ || "unknown"}</Code>
				</Text>
				<Text>
					<Link
						href="https://rivet.gg/blog/optimizing-amphibious-agility-leapfrogging-efficiency-in-code"
						target="_blank"
					>
						Ribbit!
					</Link>
				</Text>
			</Flex>
		</>
	);
}
