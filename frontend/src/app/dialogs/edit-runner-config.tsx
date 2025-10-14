import {
	useMutation,
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import * as EditRunnerConfigForm from "@/app/forms/edit-runner-config-form";
import { type DialogContentProps, Frame } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { queryClient } from "@/queries/global";

interface EditRunnerConfigFrameContentProps extends DialogContentProps {
	name: string;
	dc: string;
}

export default function EditRunnerConfigFrameContent({
	name,
	dc,
	onClose,
}: EditRunnerConfigFrameContentProps) {
	const provider = useEngineCompatDataProvider();
	usePrefetchInfiniteQuery({
		...provider.runnerConfigsQueryOptions(),
		pages: Infinity,
	});

	const { data } = useSuspenseInfiniteQuery({
		...provider.runnerConfigsQueryOptions(),
	});

	const { mutateAsync } = useMutation({
		...provider.upsertRunnerConfigMutationOptions(),
		onSuccess: () => {
			onClose?.();
		},
	});

	const config = data.find(([id]) => id === name)?.[1].datacenters?.[dc]
		.serverless;

	if (!config) {
		return (
			<Frame.Content>
				Selected provider config is not available in this datacenter.
			</Frame.Content>
		);
	}

	return (
		<EditRunnerConfigForm.Form
			onSubmit={async (values) => {
				await mutateAsync({
					name,
					config: {
						[dc]: {
							serverless: {
								...values,
								headers: Object.fromEntries(
									values.headers || [],
								),
							},
						},
					},
				});

				await queryClient.invalidateQueries(
					provider.runnerConfigsQueryOptions(),
				);
				onClose?.();
			}}
			defaultValues={{
				url: config.url,
				maxRunners: config.maxRunners,
				minRunners: config.minRunners,
				requestLifespan: config.requestLifespan,
				runnersMargin: config.runnersMargin,
				slotsPerRunner: config.slotsPerRunner,
			}}
		>
			<Frame.Header>
				<Frame.Title className="justify-between flex items-center">
					Edit {name} Provider
				</Frame.Title>
			</Frame.Header>
			<Frame.Content className="space-y-4">
				<EditRunnerConfigForm.Url />
				<div className="grid grid-cols-2 gap-2">
					<EditRunnerConfigForm.MinRunners />
					<EditRunnerConfigForm.MaxRunners />
				</div>
				<div className="grid grid-cols-2 gap-2">
					<EditRunnerConfigForm.RequestLifespan />
					<EditRunnerConfigForm.SlotsPerRunner />
				</div>

				<EditRunnerConfigForm.RunnersMargin />
				<EditRunnerConfigForm.Headers />
				<div className="flex justify-end mt-4">
					<EditRunnerConfigForm.Submit>
						Save
					</EditRunnerConfigForm.Submit>
				</div>
			</Frame.Content>
		</EditRunnerConfigForm.Form>
	);
}
