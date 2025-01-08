import { useAuth } from "@/domains/auth/contexts/auth";
import * as UserNameForm from "@/domains/group/forms/group-name-form";
import {
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
	toast,
} from "@rivet-gg/components";
import { useIdentityUpdateProfileMutation } from "../queries";

export function UserNameSettingsCard() {
	const { profile } = useAuth();
	const { mutateAsync } = useIdentityUpdateProfileMutation();
	return (
		<UserNameForm.Form
			onSubmit={async (values, form) => {
				await mutateAsync({ displayName: values.name });
				form.reset(values);
				toast.success("Profile updated");
			}}
			defaultValues={{ name: profile?.identity.displayName }}
		>
			<Card>
				<CardHeader>
					<CardTitle>Profile</CardTitle>
				</CardHeader>
				<CardContent>
					<UserNameForm.Name />
				</CardContent>
				<CardFooter>
					<UserNameForm.Submit>Save</UserNameForm.Submit>
				</CardFooter>
			</Card>
		</UserNameForm.Form>
	);
}
