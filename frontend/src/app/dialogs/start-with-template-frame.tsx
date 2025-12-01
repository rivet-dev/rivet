import { faChevronDown, faChevronLeft, Icon } from "@rivet-gg/icons";
import { templates } from "@rivetkit/example-registry";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { match } from "ts-pattern";
import { Badge, Button, type DialogContentProps, Frame } from "@/components";
import {
	type deployGroups,
	deployOptions,
} from "../../../../website/src/data/deploy/shared";
import { ExamplePreview } from "../getting-started";
import ConnectAwsFrameContent from "./connect-aws-frame";
import ConnectGcpFrameContent from "./connect-gcp-frame";
import ConnectHetznerFrameContent from "./connect-hetzner-frame";
import ConnectManualServerlfullFrameContent from "./connect-manual-serverfull-frame";
import ConnectManualServerlessFrameContent from "./connect-manual-serverless-frame";
import ConnectQuickRailwayFrameContent from "./connect-quick-railway-frame";
import ConnectQuickVercelFrameContent from "./connect-quick-vercel-frame";

interface ConnectAwsFrameContentProps extends DialogContentProps {
	name: string;
	provider?: (typeof deployGroups)[number]["items"][number]["name"];
}

export default function StartWithTemplateFrame({
	name,
	provider,
	onClose,
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
					title={`Deploy "${example.displayName}" to Vercel`}
					onClose={onClose}
					footer={footer}
				/>
			))
			.with("cloudflare", () => (
				<ConnectManualServerlessFrameContent
					provider="cloudflare-workers"
					onClose={onClose}
					footer={footer}
				/>
			))
			.with("railway", () => (
				<ConnectQuickRailwayFrameContent
					onClose={onClose}
					footer={footer}
				/>
			))
			.with("kubernetes", () => (
				<ConnectManualServerlfullFrameContent
					provider="k8s"
					onClose={onClose}
					footer={footer}
				/>
			))
			.with("aws-ecs", () => (
				<ConnectAwsFrameContent onClose={onClose} footer={footer} />
			))
			.with("gcp-cloud-run", () => (
				<ConnectGcpFrameContent onClose={onClose} footer={footer} />
			))
			.with("hetzner", () => (
				<ConnectHetznerFrameContent onClose={onClose} footer={footer} />
			))
			.with("vm-bare-metal", () => (
				<ConnectManualServerlfullFrameContent
					provider="bare-metal"
					onClose={onClose}
					footer={footer}
				/>
			))
			.otherwise(() => (
				<Frame.Content>
					<div>Provider {provider} not supported.</div>
				</Frame.Content>
			));
	}

	return (
		<>
			<ChooseProvider
				example={example}
				onProviderSelect={(provider) => {
					return navigate({
						to: ".",
						search: (old) => ({ ...old, provider }),
					});
				}}
			/>
		</>
	);
}

function ChooseProvider({
	example,
	onProviderSelect,
}: {
	example: (typeof templates)[number];
	onProviderSelect: (provider: string) => void;
}) {
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
						startIcon={<Icon icon={deployOptions[0].icon} />}
						onClick={() => onProviderSelect(deployOptions[0].name)}
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
										onProviderSelect(option.name)
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
