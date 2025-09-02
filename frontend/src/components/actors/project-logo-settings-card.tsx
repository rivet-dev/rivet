import {
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components";
import * as GroupImageForm from "@/domains/project/forms/project-logo-form";
import { useProjectLogoUploadMutation } from "../queries";

interface ProjectLogoSettingsCardProps {
	projectId: string;
}

export function ProjectLogoSettingsCard({
	projectId,
}: ProjectLogoSettingsCardProps) {
	const { mutateAsync } = useProjectLogoUploadMutation(projectId);
	return (
		<GroupImageForm.Form
			onSubmit={async (values, form) => {
				try {
					await mutateAsync({ file: values.logo });
				} catch {
					form.setError("logo", {
						type: "manual",
						message: "An error occurred while uploading the image",
					});
				}
			}}
			defaultValues={{ logo: undefined }}
		>
			<Card>
				<CardHeader>
					<CardTitle>Project Logo</CardTitle>
				</CardHeader>
				<CardContent>
					<GroupImageForm.Logo />
				</CardContent>
				<CardFooter>
					<GroupImageForm.Submit>Save</GroupImageForm.Submit>
				</CardFooter>
			</Card>
		</GroupImageForm.Form>
	);
}
