import { useAuth } from "@/domains/auth/contexts/auth";
import { useDialog } from "@/hooks/use-dialog";
import { Icon, faTriangleExclamation } from "@rivet-gg/icons";

import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	Flex,
	Strong,
	Switch,
	Text,
} from "@rivet-gg/components";
import { useIdentityDeletionMutation } from "../queries";

export function AccountDeletionCard() {
	const { profile } = useAuth();

	const { open, dialog } = useDialog.ConfirmAccountDeletion({});

	const { isPending, mutate } = useIdentityDeletionMutation();

	return (
		<>
			{dialog}
			<Card className="border-destructive">
				<CardHeader>
					<CardTitle>Danger zone</CardTitle>
				</CardHeader>
				<CardContent>
					<Flex gap="4" items="start">
						<Switch
							disabled={isPending}
							checked={profile?.identity.awaitingDeletion}
							onCheckedChange={(enabled) => {
								if (enabled) {
									open();
								} else {
									mutate(false);
								}
							}}
						/>
						<div>
							{profile?.identity.awaitingDeletion ? (
								<Flex gap="2" items="start">
									<Icon icon={faTriangleExclamation} />
									<Strong>
										Account deletion is pending.
									</Strong>
								</Flex>
							) : null}
							<Text className="my-0">
								Mark your account for deletion. After 30 days of
								this switch being on, your Rivet account and all
								associated project accounts will be{" "}
								<Strong>permanently deleted</Strong>.
							</Text>
						</div>
					</Flex>
				</CardContent>
			</Card>
		</>
	);
}
