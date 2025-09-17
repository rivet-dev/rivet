import * as EngineCredentialsForm from "@/app/forms/engine-credentials-form";
import {
	type DialogContentProps,
	Flex,
	Frame,
	getConfig,
	ls,
	toast,
} from "@/components";
import { queryClient } from "@/queries/global";
import { createClient } from "../data-providers/engine-data-provider";

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
				const client = createClient(getConfig().apiUrl, {
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
			<Frame.Header>
				<Frame.Title>Missing Rivet Engine credentials</Frame.Title>
				<Frame.Description>
					It looks like the instance of Rivet Engine that you're
					connected to requires additional credentials, please provide
					them below.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<EngineCredentialsForm.Token />
				</Flex>
			</Frame.Content>
			<Frame.Footer>
				<EngineCredentialsForm.Submit type="submit" allowPristine>
					Save
				</EngineCredentialsForm.Submit>
			</Frame.Footer>
		</EngineCredentialsForm.Form>
	);
}
