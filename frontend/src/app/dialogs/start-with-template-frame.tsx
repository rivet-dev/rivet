import { useState } from "react";
import { match } from "ts-pattern";
import { type DialogContentProps, Frame } from "@/components";
import { type Provider, TemplateProviders } from "../template-providers";
import ConnectQuickVercelFrameContent from "./connect-quick-vercel-frame";

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
		.with("cloudflare", () => {
			// TODO: Implement ConnectQuickCloudflareFrameContent
			return (
				<>
					<Frame.Header>
						<Frame.Title>Connect Cloudflare</Frame.Title>
					</Frame.Header>
					<Frame.Content>
						<div>Cloudflare connection flow goes here.</div>
					</Frame.Content>
				</>
			);
		})
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
