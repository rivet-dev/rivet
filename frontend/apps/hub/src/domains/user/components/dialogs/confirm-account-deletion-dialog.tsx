import type { DialogContentProps } from "@/hooks/use-dialog";
import {
	Button,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Strong,
	Text,
} from "@rivet-gg/components";
import { useIdentityDeletionMutation } from "../../queries";

interface ConfirmAccountDeletionDialogContentProps extends DialogContentProps {}

export default function ConfirmAccountDeletionDialogContent({
	onClose,
}: ConfirmAccountDeletionDialogContentProps) {
	const { isPending, mutate } = useIdentityDeletionMutation({
		onSuccess: onClose,
	});

	return (
		<>
			<DialogHeader>
				<DialogTitle>Confirm Account Deletion</DialogTitle>
				<DialogDescription asChild>
					<div>
						<Text>
							Are you sure you want to delete your account?
						</Text>
						<Text>
							<Strong>
								After 30 days, your account will be permanently
								deleted.
							</Strong>
						</Text>
					</div>
				</DialogDescription>
			</DialogHeader>
			<DialogFooter>
				<Button
					variant="destructive"
					type="submit"
					isLoading={isPending}
					onClick={() => {
						mutate(true);
					}}
				>
					Confirm
				</Button>
			</DialogFooter>
		</>
	);
}
