import {
	faAws,
	faChevronDown,
	faChevronLeft,
	faCloudflare,
	faGoogleCloud,
	faHetzner,
	faHetznerH,
	faKubernetes,
	faRailway,
	faServer,
	faVercel,
	Icon,
} from "@rivet-gg/icons";
import { templates } from "@rivetkit/example-registry";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { match } from "ts-pattern";
import { Badge, Button, type DialogContentProps, Frame } from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import {
	type deployGroups,
	deployOptions,
} from "../../../../website/src/data/deploy/shared";
import { ExamplePreview } from "../getting-started";
import ConnectAwsFrameContent from "./connect-aws-frame";
import ConnectCloudflareFrameContent from "./connect-cloudflare-frame";
import ConnectGcpFrameContent from "./connect-gcp-frame";
import ConnectHetznerFrameContent from "./connect-hetzner-frame";
import ConnectK8sFrameContent from "./connect-k8s-frame";
import ConnectManualServerlfullFrameContent from "./connect-manual-serverfull-frame";
import ConnectQuickRailwayFrameContent from "./connect-quick-railway-frame";
import ConnectQuickVercelFrameContent from "./connect-quick-vercel-frame";

interface ConnectAwsFrameContentProps extends DialogContentProps {
	name: string;
	provider?: (typeof deployGroups)[number]["items"][number]["name"];
	createProjectOnProviderSelect?: boolean;
}

export default function StartWithTemplateFrame({
	name,
	provider,
	onClose,
	createProjectOnProviderSelect,
}: ConnectAwsFrameContentProps) {
	const example = templates.find((t) => t.name === name);

	const navigate = useNavigate();

	if (!example) {
		return (
			<Frame.Content>
				<div>Example not found.</div>
			</Frame.Content>
		);
	}

	if (provider) {
		const footer = (
			<Button
				startIcon={<Icon icon={faChevronLeft} />}
				variant="secondary"
				onClick={() => {
					return navigate({
						to: ".",
						search: (old: any) => {
							const { provider: _, ...rest } = old;
							return rest;
						},
					});
				}}
			>
				Back
			</Button>
		);

		return match(provider)
			.with("vercel", () => (
				<ConnectQuickVercelFrameContent
					title={
						<div>
							Deploy "{example.displayName}" to{" "}
							<Icon icon={faVercel} className="ml-0.5" />
							Vercel
						</div>
					}
					onClose={onClose}
					footer={footer}
				/>
			))
			.with("cloudflare", () => (
				<ConnectCloudflareFrameContent
					title={
						<div>
							Deploy "{example.displayName}" to
							<Icon icon={faCloudflare} className="ml-0.5" />
							Cloudflare
						</div>
					}
					onClose={onClose}
					footer={footer}
				/>
			))
			.with("railway", () => (
				<ConnectQuickRailwayFrameContent
					title={
						<div>
							Deploy "{example.displayName}" to
							<Icon icon={faRailway} className="ml-0.5" />
							Railway
						</div>
					}
					onClose={onClose}
					footer={footer}
				/>
			))
			.with("kubernetes", () => (
				<ConnectK8sFrameContent
					title={
						<div>
							Deploy "{example.displayName}" to{" "}
							<Icon icon={faKubernetes} /> Kubernetes
						</div>
					}
					onClose={onClose}
					footer={footer}
				/>
			))
			.with("aws-ecs", () => (
				<ConnectAwsFrameContent
					title={
						<div>
							Deploy "{example.displayName}" to{" "}
							<Icon icon={faAws} /> AWS ECS
						</div>
					}
					onClose={onClose}
					footer={footer}
				/>
			))
			.with("gcp-cloud-run", () => (
				<ConnectGcpFrameContent
					title={
						<div>
							Deploy "{example.displayName}" to{" "}
							<Icon icon={faGoogleCloud} /> GCP Cloud Run
						</div>
					}
					onClose={onClose}
					footer={footer}
				/>
			))
			.with("hetzner", () => (
				<ConnectHetznerFrameContent
					title={
						<div>
							Deploy "{example.displayName}" to{" "}
							<Icon icon={faHetznerH} /> Hetzner
						</div>
					}
					onClose={onClose}
					footer={footer}
				/>
			))
			.with("vm-bare-metal", () => (
				<>
					<Frame.Header>
						<Frame.Title className="gap-2 flex items-center">
							<div>
								Deploy "{example.displayName}" to{" "}
								<Icon icon={faServer} /> Bare Metal / VM
							</div>
						</Frame.Title>
					</Frame.Header>
					<ConnectManualServerlfullFrameContent
						provider="bare-metal"
						onClose={onClose}
						footer={footer}
					/>
				</>
			))
			.otherwise(() => (
				<Frame.Content>
					<div>Provider {provider} not supported.</div>
				</Frame.Content>
			));
	}

	return (
		<ChooseProvider
			example={example}
			createProjectOnProviderSelect={createProjectOnProviderSelect}
		/>
	);
}

function ChooseProvider({
	example,
	createProjectOnProviderSelect,
}: {
	example: (typeof templates)[number];
	createProjectOnProviderSelect?: boolean;
}) {
	const navigate = useNavigate();

	const { mutateAsync, isPending } = useMutation(
		useCloudDataProvider().currentOrgCreateProjectMutationOptions(),
	);

	const [showProviderList, setShowProviderList] = useState(false);
	return (
		<>
			<div className="relative overflow-hidden border-b -mx-6 -mt-10">
				<ExamplePreview
					className="rounded-md"
					slug={example.name}
					title={example.displayName}
				/>

				<div className="absolute bottom-0 inset-x-0 text-center p-4">
					<h2 className="text-lg font-semibold">
						Deploy "{example.displayName}"
					</h2>
					<p className="text-sm text-muted-foreground">
						Choose your deployment provider
					</p>
				</div>
			</div>

			<Frame.Content>
				<div className="flex flex-col gap-2">
					<Button
						variant="outline"
						className="mt-4 w-full"
						isLoading={isPending}
						startIcon={<Icon icon={deployOptions[0].icon} />}
						onClick={
							createProjectOnProviderSelect
								? async () => {
										const data = await mutateAsync({
											displayName: example.displayName,
										});

										return navigate({
											to: "/orgs/$organization/projects/$project",
											params: (old) => ({
												...old,
												project: data.project.name,
											}),
											search: {
												modal: "start-with-template",
												name: example.name,
												provider: deployOptions[0].name,
											},
										});
									}
								: () =>
										navigate({
											to: ".",
											search: (old: any) => ({
												...old,
												provider: deployOptions[0].name,
											}),
										})
						}
					>
						{deployOptions[0].displayName}
						{deployOptions[0].badge ? (
							<Badge className="text-xs">
								{deployOptions[0].badge}
							</Badge>
						) : null}
					</Button>

					{!showProviderList ? (
						<Button
							variant="ghost"
							className="w-full flex-col h-auto"
							onClick={() => setShowProviderList(true)}
						>
							<div>
								More providers <Icon icon={faChevronDown} />
							</div>
							<div className="flex gap-0.5">
								{deployOptions.slice(1).map((option) => (
									<Icon
										key={option.displayName}
										icon={option.icon}
									/>
								))}
							</div>
						</Button>
					) : null}

					{showProviderList
						? deployOptions.slice(1).map((option) => (
								<Button
									key={option.displayName}
									variant="outline"
									className="w-full"
									startIcon={<Icon icon={option.icon} />}
									onClick={() =>
										navigate({
											to: ".",
											search: (old: any) => ({
												...old,
												provider: option.name,
											}),
										})
									}
								>
									{option.displayName}
									{option.badge ? (
										<Badge className="text-xs">
											{option.badge}
										</Badge>
									) : null}
								</Button>
							))
						: null}
				</div>
			</Frame.Content>
		</>
	);
}
