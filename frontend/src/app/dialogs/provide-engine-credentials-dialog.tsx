import * as EngineCredentialsForm from "@/app/forms/engine-credentials-form";
import {
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Flex,
	getConfig,
	ls,
	toast,
} from "@/components";
import type { DialogContentProps } from "@/components/actors/hooks";
import { queryClient } from "@/queries/global";
import { createClient } from "@/queries/manager-engine";

interface ProvideEngineCredentialsDialogContentProps
	extends DialogContentProps {}

export default function ProvideEngineCredentialsDialogContent({
	onClose,
}: ProvideEngineCredentialsDialogContentProps) {
	return (
		<EngineCredentialsForm.Form
			defaultValues={{ token: "" }}
			errors={
				ls.engineCredentials.get(getConfig().apiUrl)
					? { token: { message: "Invalid token.", type: "manual" } }
					: {}
			}
			onSubmit={async (values, form) => {
				const client = createClient({
					token: values.token,
				});

				try {
					await client.namespaces.list();

					ls.engineCredentials.set(getConfig().apiUrl, values.token);

					toast.success(
						"Successfully authenticated with Rivet Engine",
					);

					await queryClient.refetchQueries();

					onClose?.();
				} catch (e) {
					if (e && typeof e === "object" && "statusCode" in e) {
						if (e.statusCode === 403) {
							form.setError("token", {
								message: "Invalid token.",
							});
							return;
						}
					}

					form.setError("token", {
						message: "Failed to connect. Please try again.",
					});
					return;
				}
			}}
		>
			<DialogHeader>
				<DialogTitle>Missing Rivet Engine credentials</DialogTitle>
				<DialogDescription>
					It looks like the instance of Rivet Engine that you're
					connected to requires additional credentials, please provide
					them below.
				</DialogDescription>
			</DialogHeader>
			<Flex gap="4" direction="col">
				<EngineCredentialsForm.Token />
			</Flex>
			<DialogFooter>
				<EngineCredentialsForm.Submit type="submit" allowPristine>
					Save
				</EngineCredentialsForm.Submit>
			</DialogFooter>
		</EngineCredentialsForm.Form>
	);
}
