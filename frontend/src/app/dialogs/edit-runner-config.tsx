import type { Rivet } from "@rivetkit/engine-api-full";
import {
	useMutation,
	useSuspenseInfiniteQuery,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useFormContext } from "react-hook-form";
import * as EditRunnerConfigForm from "@/app/forms/edit-shared-runner-config-form";
import * as EditSingleRunnerConfigForm from "@/app/forms/edit-single-runner-config-form";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	Combobox,
	type DialogContentProps,
	Frame,
	ToggleGroup,
	ToggleGroupItem,
} from "@/components";
import { ActorRegion, useEngineCompatDataProvider } from "@/components/actors";
import { queryClient } from "@/queries/global";

const defaultServerlessConfig: Rivet.RunnerConfigServerless = {
	url: "",
	maxRunners: 100_000,
	requestLifespan: 300,
	slotsPerRunner: 1,
	headers: {},
};

function hasProtocolVersion(
	datacenters: Record<string, Rivet.RunnerConfigResponse>,
): boolean {
	return Object.values(datacenters).some(
		(dc) => dc.protocolVersion != null,
	);
}

function dcHasProtocolVersion(dc: Rivet.RunnerConfigResponse): boolean {
	return dc.protocolVersion != null;
}

interface EditRunnerConfigFrameContentProps extends DialogContentProps {
	name: string;
	dc?: string;
}

export default function EditRunnerConfigFrameContent({
	name,
	dc,
	onClose,
}: EditRunnerConfigFrameContentProps) {
	const provider = useEngineCompatDataProvider();

	const { data } = useSuspenseQuery({
		...provider.runnerConfigQueryOptions({ name }),
	});

	const isSharedSettings = useMemo(() => {
		const configs = Object.values(data.datacenters).map((dc) =>
			JSON.stringify(dc.serverless || {}),
		);
		return configs.every((config) => config === configs[0]);
	}, [data.datacenters]);

	const [settingsMode, setSettingsMode] = useState(
		isSharedSettings ? "shared" : "datacenter",
	);

	return (
		<Frame.Content className="gap-4 flex flex-col">
			<Frame.Header>
				<Frame.Title className="justify-between flex items-center">
					Edit '{name}' Provider
				</Frame.Title>
			</Frame.Header>

			<div>
				<SharedSettingsToggleGroup
					value={settingsMode}
					onChange={setSettingsMode}
				/>
			</div>

			<div className="gap-4 flex flex-col">
				{settingsMode === "shared" ? (
					<>
						<div className="text-sm text-muted-foreground mb-2">
							These settings will apply to all datacenters.
						</div>
						<SharedSettingsForm name={name} onClose={onClose} />
					</>
				) : null}

				{settingsMode === "datacenter" ? (
					<DatacenterSettingsForm name={name} onClose={onClose} />
				) : null}
			</div>
		</Frame.Content>
	);
}

function SharedSettingsToggleGroup({
	value,
	onChange,
}: {
	value: string;
	onChange: (mode: string) => void;
}) {
	return (
		<ToggleGroup
			defaultValue="shared"
			type="single"
			className="border rounded-md gap-0"
			value={value}
			onValueChange={(mode) => {
				if (!mode) {
					return;
				}
				onChange(mode);
			}}
		>
			<ToggleGroupItem value="shared" className="rounded-none w-full">
				Global Settings
			</ToggleGroupItem>
			<ToggleGroupItem
				value="datacenter"
				className="border-l rounded-none w-full"
			>
				Per Datacenter Settings
			</ToggleGroupItem>
		</ToggleGroup>
	);
}

function SharedSettingsForm({
	onClose,
	name,
}: {
	onClose?: () => void;
	name: string;
}) {
	const provider = useEngineCompatDataProvider();

	const { mutateAsync } = useMutation({
		...provider.upsertRunnerConfigMutationOptions(),
		onSuccess: () => {
			onClose?.();
		},
	});

	const { data } = useSuspenseQuery({
		...provider.runnerConfigQueryOptions({ name }),
	});

	const isNewConfig = hasProtocolVersion(data.datacenters);

	const currentDcConfig = Object.values(data.datacenters).find((dc) => !!dc.serverless);
	const currentServerless = currentDcConfig?.serverless ?? defaultServerlessConfig;

	return (
		<EditRunnerConfigForm.Form
			onSubmit={async ({ regions, autoUpgrade, ...values }) => {
				const serverless: Rivet.RunnerConfigServerless = isNewConfig
					? {
							url: values.url,
							requestLifespan: values.requestLifespan,
							headers: Object.fromEntries(values.headers || []),
							maxRunners: 0,
							slotsPerRunner: 0,
							maxConcurrentActors: values.maxConcurrentActors,
							drainGracePeriod: values.drainGracePeriod,
						}
					: {
							url: values.url,
							requestLifespan: values.requestLifespan,
							headers: Object.fromEntries(values.headers || []),
							maxRunners: values.maxRunners ?? 100_000,
							minRunners: values.minRunners ?? 0,
							runnersMargin: values.runnersMargin ?? 0,
							slotsPerRunner: values.slotsPerRunner ?? 1,
						};

				const config = {
					...(currentDcConfig || {}),
					serverless,
					...(isNewConfig ? { drainOnVersionUpgrade: autoUpgrade } : {}),
				};

				const providerConfig: Record<string, typeof config> = {};

				const selectedRegions = regions || {};
				for (const [regionId, isSelected] of Object.entries(
					selectedRegions,
				)) {
					if (isSelected) {
						providerConfig[regionId] = config;
					}
				}

				await mutateAsync({
					name,
					config: providerConfig,
				});

				await queryClient.invalidateQueries(
					provider.runnerConfigsQueryOptions(),
				);
				await queryClient.refetchQueries(
					provider.runnerConfigQueryOptions({ name }),
				);
				onClose?.();
			}}
			defaultValues={{
				url: currentServerless.url,
				requestLifespan: currentServerless.requestLifespan,
				headers: Object.entries(
					currentServerless.headers || {},
				).map(([key, value]) => [key, value]),
				regions: Object.fromEntries(
					Object.keys(data.datacenters).map((dcId) => [
						dcId,
						!!data.datacenters[dcId],
					]),
				),
				...(isNewConfig
					? {
							// eslint-disable-next-line @typescript-eslint/no-explicit-any
							maxConcurrentActors: (currentServerless as any).maxConcurrentActors,
							// eslint-disable-next-line @typescript-eslint/no-explicit-any
							drainGracePeriod: (currentServerless as any).drainGracePeriod,
							autoUpgrade: currentDcConfig?.drainOnVersionUpgrade ?? false,
						}
					: {
							maxRunners: currentServerless.maxRunners,
							minRunners: currentServerless.minRunners,
							runnersMargin: currentServerless.runnersMargin,
							slotsPerRunner: currentServerless.slotsPerRunner,
						}),
			}}
		>
			<EditRunnerConfigForm.Url />
			{isNewConfig ? (
				<>
					<div className="grid grid-cols-2 gap-2">
						<EditRunnerConfigForm.RequestLifespan />
						<EditRunnerConfigForm.MaxConcurrentActors />
					</div>
					<EditRunnerConfigForm.DrainGracePeriod />
					<EditRunnerConfigForm.AutoUpgrade />
				</>
			) : (
				<>
					<div className="grid grid-cols-2 gap-2">
						<EditRunnerConfigForm.MinRunners />
						<EditRunnerConfigForm.MaxRunners />
					</div>
					<div className="grid grid-cols-2 gap-2">
						<EditRunnerConfigForm.RequestLifespan />
						<EditRunnerConfigForm.SlotsPerRunner />
					</div>
					<EditRunnerConfigForm.RunnersMargin />
				</>
			)}
			<EditRunnerConfigForm.Headers />
			<EditRunnerConfigForm.Regions />
			<div className="flex justify-end mt-4">
				<EditRunnerConfigForm.Submit allowPristine>
					Save
				</EditRunnerConfigForm.Submit>
			</div>
		</EditRunnerConfigForm.Form>
	);
}

function DatacenterSettingsForm({
	onClose,
	name,
}: {
	onClose?: () => void;
	name: string;
}) {
	const provider = useEngineCompatDataProvider();

	const { data: datacenters } = useSuspenseInfiniteQuery({
		...provider.datacentersQueryOptions(),
		maxPages: Infinity,
	});

	const { mutateAsync } = useMutation({
		...provider.upsertRunnerConfigMutationOptions(),
		onSuccess: () => {
			onClose?.();
		},
	});

	const { data } = useSuspenseQuery({
		...provider.runnerConfigQueryOptions({ name }),
	});

	return (
		<EditSingleRunnerConfigForm.Form
			defaultValues={{
				datacenters: Object.fromEntries(
					Object.values(datacenters).map((dc) => [
						dc.name,
						{
							...(data.datacenters[dc.name]?.serverless ||
								defaultServerlessConfig),
							enable: !!data.datacenters[dc.name]?.serverless,
							headers: Object.entries(
								data.datacenters[dc.name]?.serverless
									?.headers || {},
							).map(([key, value]) => [key, value]),
							autoUpgrade: data.datacenters[dc.name]?.drainOnVersionUpgrade ?? false,
						},
					]),
				),
			}}
			onSubmit={async (values) => {
				const providerConfig: Record<
					string,
					{ serverless: Rivet.RunnerConfigServerless }
				> = {};

				for (const [dcId, dcConfig] of Object.entries(
					values.datacenters || {},
				)) {
					if (dcConfig?.enable) {
						const {
							enable,
							headers,
							autoUpgrade,
							...rest
						} = dcConfig;
						const isNew = dcHasProtocolVersion(
							data.datacenters[dcId] || {},
						);
						const serverless: Rivet.RunnerConfigServerless = isNew
							? {
									url: rest.url,
									requestLifespan: rest.requestLifespan,
									headers: Object.fromEntries(headers || []),
									maxRunners: 0,
									slotsPerRunner: 0,
									maxConcurrentActors: rest.maxConcurrentActors,
									drainGracePeriod: rest.drainGracePeriod,
								}
							: {
									url: rest.url,
									requestLifespan: rest.requestLifespan,
									headers: Object.fromEntries(headers || []),
									maxRunners: rest.maxRunners ?? 100_000,
									minRunners: rest.minRunners ?? 0,
									runnersMargin: rest.runnersMargin ?? 0,
									slotsPerRunner: rest.slotsPerRunner ?? 1,
								};
						providerConfig[dcId] = {
							...(data.datacenters[dcId] || {}),
							serverless,
							...(isNew ? { drainOnVersionUpgrade: autoUpgrade } : {}),
						};
					}
				}

				await mutateAsync({
					name,
					config: providerConfig,
				});

				await queryClient.invalidateQueries(
					provider.runnerConfigsQueryOptions(),
				);
				await queryClient.refetchQueries(
					provider.runnerConfigQueryOptions({ name }),
				);
				onClose?.();
			}}
		>
			<Accordion type="multiple" className="w-full">
				{datacenters.map((dc) => (
					<DatacenterAccordion
						key={dc.name}
						regionId={dc.name}
						name={name}
						data={data!}
					/>
				))}
			</Accordion>

			<EditSingleRunnerConfigForm.Datacenters />

			<div className="flex justify-end mt-4">
				<EditSingleRunnerConfigForm.Submit allowPristine>
					Save
				</EditSingleRunnerConfigForm.Submit>
			</div>
		</EditSingleRunnerConfigForm.Form>
	);
}

function DatacenterAccordion({
	regionId,
	name,
	data,
}: {
	regionId: string;
	name: string;
	data: Rivet.RunnerConfigsListResponseRunnerConfigsValue;
}) {
	const isNew = dcHasProtocolVersion(data.datacenters[regionId] || {});

	return (
		<AccordionItem value={regionId} className="-mx-2">
			<AccordionTrigger className="mx-2">
				<div className="flex items-center gap-4">
					<EditSingleRunnerConfigForm.Enable
						name={`datacenters.${regionId}.enable`}
					/>
					<ActorRegion regionId={regionId} showLabel />
				</div>
			</AccordionTrigger>
			<AccordionContent className="flex flex-col gap-4 text-balance px-2">
				<SelectDatacenterSettingsSource
					name={name}
					currentRegionId={regionId}
				/>

				<EditSingleRunnerConfigForm.Url
					name={`datacenters.${regionId}.url`}
				/>
				{isNew ? (
					<>
						<div className="grid grid-cols-2 gap-2">
							<EditSingleRunnerConfigForm.RequestLifespan
								name={`datacenters.${regionId}.requestLifespan`}
							/>
							<EditSingleRunnerConfigForm.MaxConcurrentActors
								name={`datacenters.${regionId}.maxConcurrentActors`}
							/>
						</div>
						<EditSingleRunnerConfigForm.DrainGracePeriod
							name={`datacenters.${regionId}.drainGracePeriod`}
						/>
						<EditSingleRunnerConfigForm.AutoUpgrade
							name={`datacenters.${regionId}.autoUpgrade`}
						/>
					</>
				) : (
					<>
						<div className="grid grid-cols-2 gap-2">
							<EditSingleRunnerConfigForm.MinRunners
								name={`datacenters.${regionId}.minRunners`}
							/>
							<EditSingleRunnerConfigForm.MaxRunners
								name={`datacenters.${regionId}.maxRunners`}
							/>
						</div>
						<div className="grid grid-cols-2 gap-2">
							<EditSingleRunnerConfigForm.RequestLifespan
								name={`datacenters.${regionId}.requestLifespan`}
							/>
							<EditSingleRunnerConfigForm.SlotsPerRunner
								name={`datacenters.${regionId}.slotsPerRunner`}
							/>
						</div>
						<EditSingleRunnerConfigForm.RunnersMargin
							name={`datacenters.${regionId}.runnersMargin`}
						/>
					</>
				)}
				<EditSingleRunnerConfigForm.Headers
					name={`datacenters.${regionId}.headers`}
				/>
			</AccordionContent>
		</AccordionItem>
	);
}

function SelectDatacenterSettingsSource({
	currentRegionId,
	name,
}: {
	currentRegionId: string;
	name: string;
}) {
	const form = useFormContext();
	const provider = useEngineCompatDataProvider();

	const { data: datacenters } = useSuspenseInfiniteQuery({
		...provider.datacentersQueryOptions(),
		maxPages: Infinity,
	});

	const { data: runnerConfig } = useSuspenseQuery({
		...provider.runnerConfigQueryOptions({ name }),
	});

	const availableDatacenters = datacenters.filter(
		(dc) =>
			dc.name !== currentRegionId && runnerConfig.datacenters[dc.name],
	);

	if (availableDatacenters.length === 0) {
		return null;
	}

	return (
		<div className="flex gap-2 items-center text-muted-foreground">
			Copy settings from
			<Combobox
				onValueChange={(selectedRegionId) => {
					form.setValue(
						`datacenters.${currentRegionId}`,
						form.getValues(`datacenters.${selectedRegionId}`),
						{ shouldDirty: true, shouldTouch: true },
					);
				}}
				value={undefined}
				placeholder="Select datacenter"
				className="w-auto min-w-[200px]"
				options={availableDatacenters.map((dc) => ({
					value: dc.name,
					label: <ActorRegion regionId={dc.name} showLabel />,
				}))}
			/>
		</div>
	);
}
