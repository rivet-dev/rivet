import type { Story } from "@ladle/react";
import "../../.ladle/ladle.css";
import { DiscreteInput, TooltipProvider } from "@/components";
import { Label } from "@/components/ui/label";
import { PublishableTokenNotice } from "./env-variables";

function Frame({ children }: { children: React.ReactNode }) {
	return (
		<TooltipProvider>
			<div className="bg-background min-h-screen p-12">
				<div className="max-w-3xl space-y-10">{children}</div>
			</div>
		</TooltipProvider>
	);
}

function Section({
	title,
	children,
}: {
	title: string;
	children: React.ReactNode;
}) {
	return (
		<div className="space-y-2">
			<h3 className="text-sm font-medium text-muted-foreground">
				{title}
			</h3>
			<div className="rounded-md border bg-card p-4">{children}</div>
		</div>
	);
}

const NS = "localhost-ftji-production-ualj";
const ADMIN_TOKEN = "sk_Km9XaQpL2tRZ3nVxYbCdEfGhJkMnPqRsTuVwXyZaBcDe";
const PUBLISHABLE_TOKEN = "pk_ivU6QmAj3sKwLp1XnVxYbCdEfGhJkMnPqRsTuVwXyZ";
const HOST = "engine.example.com";

function MockEnvBlock({ publicDsn, secretDsn }: { publicDsn: string; secretDsn: string }) {
	return (
		<div className="gap-1 items-center grid grid-cols-2">
			<Label asChild className="text-muted-foreground text-xs mb-1">
				<p>Key</p>
			</Label>
			<Label asChild className="text-muted-foreground text-xs mb-1">
				<p>Value</p>
			</Label>
			<DiscreteInput
				aria-label="environment variable key"
				value="RIVET_PUBLIC_ENDPOINT"
				show
			/>
			<DiscreteInput
				aria-label="environment variable value"
				value={publicDsn}
				show
			/>
			<DiscreteInput
				aria-label="environment variable key"
				value="RIVET_ENDPOINT"
				show
			/>
			<DiscreteInput
				aria-label="environment variable value"
				value={secretDsn}
				show
			/>
		</div>
	);
}

export const PlainOss: Story = () => (
	<Frame>
		<Section title="Plain OSS — no auth, namespace-only public DSN">
			<div className="space-y-3">
				<MockEnvBlock
					publicDsn={`https://${NS}@${HOST}`}
					secretDsn={`https://${NS}:${ADMIN_TOKEN}@${HOST}`}
				/>
			</div>
		</Section>
	</Frame>
);

export const Cloud: Story = () => (
	<Frame>
		<Section title="Cloud — fetched publishable token">
			<div className="space-y-3">
				<MockEnvBlock
					publicDsn={`https://${NS}:${PUBLISHABLE_TOKEN}@${HOST}`}
					secretDsn={`https://${NS}:${ADMIN_TOKEN}@${HOST}`}
				/>
			</div>
		</Section>
	</Frame>
);

export const EnterpriseAcl: Story = () => (
	<Frame>
		<Section title="Enterprise (ACL, no platform API) — placeholder + RBAC notice">
			<div className="space-y-3">
				<PublishableTokenNotice />
				<MockEnvBlock
					publicDsn={`https://${NS}:<PUBLISHABLE_TOKEN>@${HOST}`}
					secretDsn={`https://${NS}:${ADMIN_TOKEN}@${HOST}`}
				/>
			</div>
		</Section>
	</Frame>
);

export const NoticeOnly: Story = () => (
	<Frame>
		<Section title="Publishable token notice in isolation">
			<PublishableTokenNotice />
		</Section>
	</Frame>
);

export const Gallery: Story = () => (
	<Frame>
		<Section title="Plain OSS">
			<MockEnvBlock
				publicDsn={`https://${NS}@${HOST}`}
				secretDsn={`https://${NS}:${ADMIN_TOKEN}@${HOST}`}
			/>
		</Section>
		<Section title="Cloud">
			<MockEnvBlock
				publicDsn={`https://${NS}:${PUBLISHABLE_TOKEN}@${HOST}`}
				secretDsn={`https://${NS}:${ADMIN_TOKEN}@${HOST}`}
			/>
		</Section>
		<Section title="Enterprise (ACL)">
			<div className="space-y-3">
				<PublishableTokenNotice />
				<MockEnvBlock
					publicDsn={`https://${NS}:<PUBLISHABLE_TOKEN>@${HOST}`}
					secretDsn={`https://${NS}:${ADMIN_TOKEN}@${HOST}`}
				/>
			</div>
		</Section>
	</Frame>
);
