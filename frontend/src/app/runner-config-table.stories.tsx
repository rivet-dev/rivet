import type { Story } from "@ladle/react";
import type { Rivet } from "@rivetkit/engine-api-full";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "../../.ladle/ladle.css";
import { TooltipProvider } from "@/components";
import {
	getRegionLabel,
	RegionIcon,
} from "@/components/matchmaker/lobby-region";
import { RunnerConfigsTable } from "./runner-config-table";

const queryClient = new QueryClient({
	defaultOptions: { queries: { retry: false, staleTime: Infinity } },
});

function Frame({ children }: { children: React.ReactNode }) {
	return (
		<QueryClientProvider client={queryClient}>
			<TooltipProvider>
				<div className="bg-background min-h-screen p-12">
					<div className="max-w-5xl mx-auto border rounded-md">
						{children}
					</div>
				</div>
			</TooltipProvider>
		</QueryClientProvider>
	);
}

const serverless = (
	url: string,
	provider: string,
	extra: Partial<Rivet.RunnerConfigResponse> = {},
): Rivet.RunnerConfigResponse => ({
	serverless: {
		url,
		headers: {},
		requestLifespan: 300,
		runnersMargin: 1,
		minRunners: 0,
		maxRunners: 10,
		slotsPerRunner: 100,
	} as Rivet.RunnerConfigServerless,
	metadata: { provider },
	...extra,
});

const serverful = (
	provider: string,
	extra: Partial<Rivet.RunnerConfigResponse> = {},
): Rivet.RunnerConfigResponse => ({
	normal: {},
	metadata: { provider },
	...extra,
});

const renderRegion = (
	regionId: string,
	{ abbreviated }: { abbreviated?: boolean },
) => (
	<span className="inline-flex items-center gap-2 whitespace-nowrap">
		<RegionIcon region={regionId} className="size-4 min-w-4" />
		<span>
			{abbreviated ? regionId.toUpperCase() : getRegionLabel(regionId)}
		</span>
	</span>
);

const tableProps = {
	renderRegion,
	onEditConfig: (name: string) => console.log("edit", name),
	onDeleteConfig: (name: string) => console.log("delete", name),
};

export const SingleProviderServerlessOneEndpoint: Story = () => (
	<Frame>
		<RunnerConfigsTable
			{...tableProps}
			totalDatacenterCount={3}
			configs={[
				[
					"api",
					{
						datacenters: {
							atl: serverless("https://api.vercel.app/_rivet", "vercel"),
						},
					},
				],
			]}
		/>
	</Frame>
);

export const SingleProviderServerlessMultipleEndpoints: Story = () => (
	<Frame>
		<RunnerConfigsTable
			{...tableProps}
			totalDatacenterCount={3}
			configs={[
				[
					"api",
					{
						datacenters: {
							atl: serverless(
								"https://api-atl.vercel.app/_rivet",
								"vercel",
							),
							fra: serverless(
								"https://api-fra.vercel.app/_rivet",
								"vercel",
							),
							syd: serverless(
								"https://api-syd.vercel.app/_rivet",
								"vercel",
							),
						},
					},
				],
			]}
		/>
	</Frame>
);

export const AllServerful: Story = () => (
	<Frame>
		<RunnerConfigsTable
			{...tableProps}
			totalDatacenterCount={3}
			configs={[
				[
					"default",
					{
						datacenters: {
							atl: serverful("custom"),
							fra: serverful("custom"),
							syd: serverful("custom"),
						},
					},
				],
			]}
		/>
	</Frame>
);

export const MixedKinds: Story = () => (
	<Frame>
		<RunnerConfigsTable
			{...tableProps}
			totalDatacenterCount={4}
			configs={[
				[
					"hybrid",
					{
						datacenters: {
							atl: serverless(
								"https://api.vercel.app/_rivet",
								"vercel",
							),
							fra: serverful("hetzner"),
							syd: serverless(
								"https://api-syd.vercel.app/_rivet",
								"vercel",
							),
						},
					},
				],
			]}
		/>
	</Frame>
);

export const SameProviderMixedKinds: Story = () => (
	<Frame>
		<RunnerConfigsTable
			{...tableProps}
			totalDatacenterCount={3}
			configs={[
				[
					"vercel-hybrid",
					{
						datacenters: {
							atl: serverless(
								"https://api.vercel.app/_rivet",
								"vercel",
							),
							fra: serverful("vercel"),
						},
					},
				],
			]}
		/>
	</Frame>
);

export const MultipleProviders: Story = () => (
	<Frame>
		<RunnerConfigsTable
			{...tableProps}
			totalDatacenterCount={3}
			configs={[
				[
					"multi-cloud",
					{
						datacenters: {
							atl: serverless(
								"https://api.vercel.app/_rivet",
								"vercel",
							),
							fra: serverless(
								"https://api.railway.app/_rivet",
								"railway",
							),
						},
					},
				],
			]}
		/>
	</Frame>
);

export const GlobalDeployment: Story = () => (
	<Frame>
		<RunnerConfigsTable
			{...tableProps}
			totalDatacenterCount={3}
			configs={[
				[
					"global-api",
					{
						datacenters: {
							atl: serverless(
								"https://api.vercel.app/_rivet",
								"vercel",
							),
							fra: serverless(
								"https://api.vercel.app/_rivet",
								"vercel",
							),
							syd: serverless(
								"https://api.vercel.app/_rivet",
								"vercel",
							),
						},
					},
				],
			]}
		/>
	</Frame>
);

export const WithRunnerPoolError: Story = () => (
	<Frame>
		<RunnerConfigsTable
			{...tableProps}
			totalDatacenterCount={3}
			configs={[
				[
					"api",
					{
						datacenters: {
							atl: serverless(
								"https://api.vercel.app/_rivet",
								"vercel",
								{
									runnerPoolError: {
										serverless_http_error: {
											status_code: 500,
											body: "Internal Server Error",
										},
									},
								},
							),
							fra: serverless(
								"https://api.vercel.app/_rivet",
								"vercel",
							),
						},
					},
				],
			]}
		/>
	</Frame>
);

export const Loading: Story = () => (
	<Frame>
		<RunnerConfigsTable {...tableProps} isLoading configs={[]} />
	</Frame>
);

export const Empty: Story = () => (
	<Frame>
		<RunnerConfigsTable {...tableProps} configs={[]} />
	</Frame>
);

export const Error: Story = () => (
	<Frame>
		<RunnerConfigsTable {...tableProps} isError configs={[]} />
	</Frame>
);

export const Gallery: Story = () => (
	<Frame>
		<RunnerConfigsTable
			{...tableProps}
			totalDatacenterCount={4}
			configs={[
				[
					"api-serverless",
					{
						datacenters: {
							atl: serverless(
								"https://api.vercel.app/_rivet",
								"vercel",
							),
						},
					},
				],
				[
					"default-serverful",
					{
						datacenters: {
							atl: serverful("custom"),
							fra: serverful("custom"),
						},
					},
				],
				[
					"hybrid-mixed",
					{
						datacenters: {
							atl: serverless(
								"https://api.vercel.app/_rivet",
								"vercel",
							),
							fra: serverful("hetzner"),
						},
					},
				],
				[
					"same-provider-mixed-kinds",
					{
						datacenters: {
							atl: serverless(
								"https://api.vercel.app/_rivet",
								"vercel",
							),
							fra: serverful("vercel"),
						},
					},
				],
				[
					"multi-endpoints",
					{
						datacenters: {
							atl: serverless(
								"https://api-atl.vercel.app/_rivet",
								"vercel",
							),
							fra: serverless(
								"https://api-fra.vercel.app/_rivet",
								"vercel",
							),
						},
					},
				],
				[
					"with-error",
					{
						datacenters: {
							atl: serverless(
								"https://api.vercel.app/_rivet",
								"vercel",
								{
									runnerPoolError: {
										serverless_connection_error: {
											message: "connection refused",
										},
									},
								},
							),
						},
					},
				],
			]}
		/>
	</Frame>
);
