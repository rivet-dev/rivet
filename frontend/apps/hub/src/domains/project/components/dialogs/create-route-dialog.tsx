import * as EditRouteForm from "@/domains/project/forms/route-edit-form";
import type { DialogContentProps } from "@/hooks/use-dialog";
import {
	Button,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Label,
} from "@rivet-gg/components";
import { useState } from "react";
import { useCreateRouteMutation } from "../../queries";

interface ContentProps extends DialogContentProps {
	projectNameId: string;
	environmentNameId: string;
}

export default function CreateRouteDialogContent(props: ContentProps) {
	return <Content {...props} />;
}

function Content({ projectNameId, environmentNameId, onClose }: ContentProps) {
	const { mutateAsync } = useCreateRouteMutation();

	const [tagKeys, setTagKeys] = useState<{ label: string; value: string }[]>(
		() => [],
	);

	const [tagValues, setTagValues] = useState<
		{ label: string; value: string }[]
	>(() => []);

	return (
		<EditRouteForm.Form
			defaultValues={{
				tags: [],
				hostname: "",
				path: "",
				stripPrefix: false,
				routeSubpaths: false,
			}}
			onSubmit={async (values) => {
				const selectorTags = Object.fromEntries(
					values.tags.map(({ key, value }) => [key, value]),
				);

				await mutateAsync({
					projectNameId,
					environmentNameId,
					hostname: values.hostname,
					path: values.path,
					stripPrefix: values.stripPrefix || false,
					routeSubpaths: values.routeSubpaths || false,
					target: {
						actors: {
							selectorTags,
						},
					},
				});
				onClose?.();
			}}
		>
			<DialogHeader>
				<DialogTitle>Add Route</DialogTitle>
			</DialogHeader>
			<div className="flex gap-2">
				<EditRouteForm.Hostname />
				<EditRouteForm.Path />
			</div>

			<Label>Selectors</Label>

			<EditRouteForm.Tags
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

			<EditRouteForm.RouteSubpaths />
			<EditRouteForm.StripPrefix />

			<DialogFooter>
				<EditRouteForm.Submit>Save</EditRouteForm.Submit>
				<Button type="button" variant="secondary" onClick={onClose}>
					Close
				</Button>
			</DialogFooter>
		</EditRouteForm.Form>
	);
}
