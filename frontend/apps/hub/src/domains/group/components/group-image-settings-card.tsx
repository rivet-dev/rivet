import * as GroupImageForm from "@/domains/group/forms/group-image-form";
import { useAvatarUploadMutation } from "@/domains/group/queries";
import {
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
	Skeleton,
} from "@rivet-gg/components";

interface GroupImageSettingsCardProps {
	groupId: string;
}

export function GroupImageSettingsCard({
	groupId,
}: GroupImageSettingsCardProps) {
	const { mutateAsync } = useAvatarUploadMutation(groupId);
	return (
		<GroupImageForm.Form
			onSubmit={async (values, form) => {
				try {
					await mutateAsync({ file: values.image });
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
					<CardTitle>Team Image</CardTitle>
				</CardHeader>
				<CardContent>
					<GroupImageForm.Image />
				</CardContent>
				<CardFooter>
					<GroupImageForm.Submit>Save</GroupImageForm.Submit>
				</CardFooter>
			</Card>
		</GroupImageForm.Form>
	);
}

GroupImageSettingsCard.Skeleton = function GroupImageSettingsCard() {
	return <Skeleton className="w-full h-56" />;
};
