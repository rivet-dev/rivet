import {
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
	toast,
} from "@rivet-gg/components";
import * as UserAvatarForm from "../forms/user-avatar-form";
import { useAvatarUploadMutation } from "../queries";

export function UserAvatarSettingsCard() {
	const { mutateAsync } = useAvatarUploadMutation();
	return (
		<UserAvatarForm.Form
			onSubmit={async (values, form) => {
				try {
					await mutateAsync({ file: values.image });
					form.reset();
					toast.success("Avatar updated");
				} catch {
					form.setError("image", {
						type: "manual",
						message: "An error occurred while uploading the image",
					});
				}
			}}
			defaultValues={{ image: undefined }}
		>
			<Card>
				<CardHeader>
					<CardTitle>Avatar</CardTitle>
				</CardHeader>
				<CardContent>
					<UserAvatarForm.Image />
				</CardContent>
				<CardFooter>
					<UserAvatarForm.Submit>Save</UserAvatarForm.Submit>
				</CardFooter>
			</Card>
		</UserAvatarForm.Form>
	);
}
