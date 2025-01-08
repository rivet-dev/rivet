import * as GroupNameForm from "@/domains/group/forms/group-name-form";
import { useGroupUpdateProfileMutation } from "@/domains/group/queries";
import { groupProjectsQueryOptions } from "@/domains/project/queries";
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
	Skeleton,
} from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";

interface GroupNameSettingsCardProps {
	groupId: string;
}

export function GroupNameSettingsCard({ groupId }: GroupNameSettingsCardProps) {
	const { data } = useSuspenseQuery(groupProjectsQueryOptions(groupId));
	const { mutateAsync } = useGroupUpdateProfileMutation();
	return (
		<GroupNameForm.Form
			onSubmit={(values) => {
				return mutateAsync({ groupId, displayName: values.name });
			}}
			defaultValues={{ name: data?.displayName }}
		>
			<Card>
				<CardHeader>
					<CardTitle>Team Name</CardTitle>
					<CardDescription>
						Used to identify your team in various parts of the
						ecosystem.
					</CardDescription>
				</CardHeader>
				<CardContent>
					<GroupNameForm.Name />
				</CardContent>
				<CardFooter>
					<GroupNameForm.Submit>Save</GroupNameForm.Submit>
				</CardFooter>
			</Card>
		</GroupNameForm.Form>
	);
}

GroupNameSettingsCard.Skeleton = function GroupNameSettingsCardSkeleton() {
	return <Skeleton className="w-full h-56" />;
};
