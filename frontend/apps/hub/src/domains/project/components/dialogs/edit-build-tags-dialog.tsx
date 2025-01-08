import * as EditBuildTagsForm from "@/domains/project/forms/build-tags-form";
import type { DialogContentProps } from "@/hooks/use-dialog";
import {
	Button,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@rivet-gg/components";
import { useSuspenseQueries } from "@tanstack/react-query";
import { useState } from "react";
import {
	actorBuildQueryOptions,
	actorBuildsQueryOptions,
	usePatchActorBuildTagsMutation,
} from "../../queries";

interface ContentProps extends DialogContentProps {
	projectNameId: string;
	environmentNameId: string;
	buildId: string;
}

export default function EditBuildTagsDialogContent(props: ContentProps) {
	if (!props.buildId) {
		return null;
	}

	return <Content {...props} />;
}

function Content({
	buildId,
	projectNameId,
	environmentNameId,
	onClose,
}: ContentProps) {
	const [{ data }, { data: builds }] = useSuspenseQueries({
		queries: [
			actorBuildQueryOptions({
				buildId,
				projectNameId,
				environmentNameId,
			}),
			actorBuildsQueryOptions({ projectNameId, environmentNameId }),
		],
	});
	const { mutateAsync } = usePatchActorBuildTagsMutation();

	const [tagKeys, setTagKeys] = useState(() =>
		Array.from(
			new Set(
				builds?.flatMap((build) => Object.keys(build.tags)),
			).values(),
		).map((key) => ({
			label: key,
			value: key,
		})),
	);

	const [tagValues, setTagValues] = useState(() =>
		Array.from(
			new Set(
				builds?.flatMap((build) => Object.values(build.tags)),
			).values(),
		).map((key) => ({ label: key, value: key })),
	);

	return (
		<EditBuildTagsForm.Form
			defaultValues={{
				tags: Object.entries(data.tags).map(([key, value]) => ({
					key,
					value,
				})),
			}}
			onSubmit={async (values) => {
				const tags = Object.fromEntries(
					values.tags.map(({ key, value }) => [key, value]),
				);

				await mutateAsync({
					projectNameId,
					environmentNameId,
					buildId,
					tags,
				});
				onClose?.();
			}}
		>
			<DialogHeader>
				<DialogTitle>Edit Build Tags</DialogTitle>
			</DialogHeader>

			<EditBuildTagsForm.Tags
				keys={tagKeys}
				values={tagValues}
				onCreateKeyOption={(option) =>
					setTagKeys((opts) => [
						...opts,
						{ label: option, value: option },
					])
				}
				onCreateValueOption={(option) =>
					setTagValues((opts) => [
						...opts,
						{ label: option, value: option },
					])
				}
			/>

			<DialogFooter>
				<EditBuildTagsForm.Submit>Save</EditBuildTagsForm.Submit>
				<Button type="button" variant="secondary" onClick={onClose}>
					Close
				</Button>
			</DialogFooter>
		</EditBuildTagsForm.Form>
	);
}
