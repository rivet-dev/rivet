import * as EditRouteForm from "@/domains/project/forms/route-edit-form";
import type { DialogContentProps } from "@/hooks/use-dialog";
import {
	Button,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Label,
} from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { useState } from "react";
import { routeQueryOptions, usePatchRouteMutation } from "../../queries";
import type { Rivet } from "@rivet-gg/api";

interface OptionalContentProps extends DialogContentProps {
	projectNameId: string;
	environmentNameId: string;
	routeId?: string;
}

export default function EditRouteDialogContent(props: OptionalContentProps) {
	if (!props.routeId) {
		return null;
	}

	return <GuardedContent {...props} routeId={props.routeId} />;
}

function GuardedContent({
	routeId,
	projectNameId,
	environmentNameId,
	onClose,
}: Omit<OptionalContentProps, "routeNameId"> & { routeId: string }) {
	const { data } = useSuspenseQuery(
		routeQueryOptions({
			routeId,
			projectNameId,
			environmentNameId,
		}),
	);

	if (!data) {
		return (
			<>
				<div className="flex flex-col gap-2">
					<p>Route not found</p>
					<p className="text-sm ">
						The route you are trying to edit does not exist.
					</p>
				</div>

				<DialogFooter>
					<Button type="button" variant="secondary" onClick={onClose}>
						Close
					</Button>
				</DialogFooter>
			</>
		);
	}

	return (
		<Content
			{...data}
			projectNameId={projectNameId}
			environmentNameId={environmentNameId}
			onClose={onClose}
		/>
	);
}

interface ContentProps extends DialogContentProps, Rivet.routes.Route {
	projectNameId: string;
	environmentNameId: string;
}

function Content({
	projectNameId,
	id,
	target,
	environmentNameId,
	hostname,
	routeSubpaths,
	stripPrefix,
	path,
	onClose,
}: ContentProps) {
	const { mutateAsync } = usePatchRouteMutation();

	const tags = Object.entries(target?.actors?.selectorTags || {}).map(
		([key, value]) => ({
			key,
			value,
		}),
	);

	const [tagKeys, setTagKeys] = useState<{ label: string; value: string }[]>(
		() => tags.map(({ key }) => ({ label: key, value: key })),
	);

	const [tagValues, setTagValues] = useState<
		{ label: string; value: string }[]
	>(() => tags.map(({ value }) => ({ label: value, value })));

	return (
		<EditRouteForm.Form
			defaultValues={{
				tags: Object.entries(target?.actors?.selectorTags || {}).map(
					([key, value]) => ({
						key,
						value,
					}),
				),
				hostname,
				path,
				routeSubpaths,
				stripPrefix,
			}}
			onSubmit={async (values) => {
				const selectorTags = Object.fromEntries(
					values.tags.map(({ key, value }) => [key, value]),
				);

				await mutateAsync({
					projectNameId,
					environmentNameId,
					id,
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
