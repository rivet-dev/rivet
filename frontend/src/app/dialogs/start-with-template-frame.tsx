import { useState } from "react";
import { match } from "ts-pattern";
import { type DialogContentProps, Frame } from "@/components";
import { type Provider, TemplateProviders } from "../template-providers";
import ConnectQuickVercelFrameContent from "./connect-quick-vercel-frame";
import ConnectQuickRailwayFrameContent from "./connect-quick-railway-frame";
import ConnectAwsFrameContent from "./connect-aws-frame";
import ConnectGcpFrameContent from "./connect-gcp-frame";
import ConnectHetznerFrameContent from "./connect-hetzner-frame";
import ConnectManualServerlfullFrameContent from "./connect-manual-serverfull-frame";
import ConnectManualServerlessFrameContent from "./connect-manual-serverless-frame";

interface ConnectAwsFrameContentProps extends DialogContentProps {
	template: string;
}

export default function StartWithTemplateFrame({
	template,
	onClose,
}: ConnectAwsFrameContentProps) {
	const [provider, setProvider] = useState<Provider | null>(null);

	return match(provider)
		.with("vercel", () => (
			<ConnectQuickVercelFrameContent onClose={onClose} />
		))
		.with("cloudflare", () => <ConnectManualServerlessFrameContent provider="cloudflare-workers" onClose={onClose} />)
        .with("railway", () => <ConnectQuickRailwayFrameContent onClose={onClose} />)
        .with("kubernetes", () => <ConnectManualServerlfullFrameContent provider="k8s" onClose={onClose} />)
        .with("aws-ecs", () => <ConnectAwsFrameContent onClose={onClose} />)
        .with("gcp-cloud-run", () =>  <ConnectGcpFrameContent onClose={onClose} />)
        .with("hetzner", () => <ConnectHetznerFrameContent onClose={onClose} />)
        .with("vm-bare-metal", () => <ConnectManualServerlfullFrameContent provider="bare-metal" onClose={onClose} />)
		.otherwise(() => {
			return (
				<>
					<Frame.Header>
						<Frame.Title className="gap-2 flex items-center">
							<div>Start With {template}</div>
						</Frame.Title>
					</Frame.Header>
					<Frame.Content>
						<div className="mb-4 font-semibold">
							Select Provider
						</div>
						<TemplateProviders onProviderSelect={setProvider} />
					</Frame.Content>
				</>
			);
		});
}
