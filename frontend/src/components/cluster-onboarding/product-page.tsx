import {
	faArrowUpRightFromSquare,
	faBug,
	faCopy,
	faFileLines,
	faGlobe,
	faKey,
	faRectangleTerminal,
	Icon,
} from "@rivet-gg/icons";
import { useMemo, useState } from "react";
import { CodePreview } from "@/components/code-preview/code-preview";
import { CopyTrigger, DiscreteInput } from "@/components/copy-area";
import { cn } from "@/components/lib/utils";
import { ProductMark } from "@/components/products/product-picker";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ActorMetricsDashboard, mockActorMetrics } from "./actor-metrics";
import { AgentConnect } from "./agent-connect";
import { Endpoint, EndpointTitle } from "./connection-fields";
import type { ClusterEndpoints } from "./model";
import {
	type DocLink,
	type ProductPageContent,
	productAgentPrompt,
	QUICKSTART_LANGS,
	type QuickstartLang,
	type ResolvedStep,
	resolveQuickstart,
} from "./product-content";

/**
 * Product page: header with the product mark, the endpoint and token every
 * snippet uses, a quickstart that mirrors the product's rivet.dev quickstart
 * with a TypeScript/HTTP toggle, and a right rail of guides and API links.
 */
export function ProductPage({
	product,
	endpoints,
	namespace,
	onboarded,
	onOnboardedChange,
}: {
	product: ProductPageContent;
	endpoints: ClusterEndpoints;
	namespace: string;
	/** Debug toggle: show the post-setup metrics dashboard instead of the quickstart. */
	onboarded: boolean;
	onOnboardedChange: (onboarded: boolean) => void;
}) {
	const [lang, setLang] = useState<QuickstartLang>("typescript");
	const metrics = useMemo(() => mockActorMetrics(product.id), [product.id]);
	const steps = useMemo(
		() =>
			resolveQuickstart(product, lang, {
				endpoint: endpoints.external.url,
				token: endpoints.adminToken,
				namespace,
			}),
		[product, lang, endpoints, namespace],
	);

	return (
		<div className="flex flex-col gap-8">
			<header className="flex flex-wrap items-start justify-between gap-4">
				<div className="flex min-w-0 items-start gap-4">
					<div className="shrink-0 pt-0.5">
						<ProductMark
							fileName={product.markFileName}
							section={product.section}
							size="md"
						/>
					</div>
					<div className="min-w-0">
						<h1 className="text-2xl font-semibold leading-tight">
							{product.label}
						</h1>
						<p className="mt-1 max-w-2xl text-sm text-muted-foreground">
							{product.description}
						</p>
					</div>
				</div>
			</header>

			{onboarded ? null : (
				<AgentConnect
					prompt={productAgentPrompt(product)}
					label={`Connect your agent to get started with ${product.label}`}
					hint={`Copied. Paste it into your agent — it sets up Rivet, walks you through ${product.label}, runs it locally, then deploys here.`}
				/>
			)}

			<div className="flex items-start gap-8">
				{onboarded ? (
					<div className="min-w-0 flex-1">
						<ActorMetricsDashboard metrics={metrics} />
					</div>
				) : (
					<div className="flex min-w-0 flex-1 flex-col gap-6">
						<Card>
							<CardContent className="grid gap-5 pt-6 lg:grid-cols-2">
								<Endpoint
									icon={faGlobe}
									title="Endpoint"
									url={endpoints.external.url}
								/>
								<div className="flex flex-col gap-2">
									<EndpointTitle icon={faKey} title="Token" />
									<DiscreteInput
										value={endpoints.adminToken}
									/>
								</div>
							</CardContent>
						</Card>

						<div className="rounded-lg border bg-background/40 p-6">
							<div className="flex flex-wrap items-center justify-between gap-4">
								<h2 className="text-lg font-semibold">
									Quickstart
								</h2>
								<Tabs
									value={lang}
									onValueChange={(v) =>
										setLang(v as QuickstartLang)
									}
								>
									<TabsList className="w-auto border-0">
										{QUICKSTART_LANGS.map((l) => (
											<TabsTrigger
												key={l.id}
												value={l.id}
												className="h-8 px-3 pb-2 pt-1.5"
											>
												{l.label}
											</TabsTrigger>
										))}
									</TabsList>
								</Tabs>
							</div>

							<ol className="mt-6 flex flex-col">
								{steps.map((step, i) => (
									<StepRow
										key={step.title}
										index={i + 1}
										step={step}
										last={i === steps.length - 1}
									/>
								))}
							</ol>
						</div>
					</div>
				)}

				<aside className="hidden w-72 shrink-0 flex-col gap-8 lg:flex">
					<LinkList
						title="Guides"
						links={product.guides}
						moreUrl={product.guidesUrl}
					/>
					<LinkList
						title="API reference"
						links={product.apiReference}
						moreUrl={product.apiDocsUrl}
						mono
					/>
				</aside>
			</div>

			<div className="flex justify-end border-t pt-4">
				<Button
					variant="ghost"
					size="sm"
					className="text-muted-foreground"
					onClick={() => onOnboardedChange(!onboarded)}
				>
					<Icon icon={faBug} />
					Debug: switch to {onboarded ? "setup" : "onboarded"} view
				</Button>
			</div>
		</div>
	);
}

function StepRow({
	index,
	step,
	last,
}: {
	index: number;
	step: ResolvedStep;
	last: boolean;
}) {
	return (
		<li className="relative flex gap-4 pb-8 last:pb-0">
			{last ? null : (
				<span
					aria-hidden="true"
					className="absolute left-3.5 top-8 bottom-0 w-px bg-border"
				/>
			)}
			<span className="relative z-10 flex size-7 shrink-0 items-center justify-center rounded-full border bg-card text-xs font-medium tabular-nums">
				{index}
			</span>
			<div className="min-w-0 flex-1 pt-0.5">
				<p className="text-sm font-medium">{step.title}</p>
				{step.description ? (
					<p className="mt-1 text-sm text-muted-foreground">
						{step.description}
					</p>
				) : null}
				{step.blocks.map((block) => (
					<CodeBlock key={block.code} block={block} />
				))}
				{step.after ? (
					<p className="mt-3 text-sm text-muted-foreground">
						{step.after}
					</p>
				) : null}
			</div>
		</li>
	);
}

function CodeBlock({ block }: { block: ResolvedStep["blocks"][number] }) {
	return (
		<div className="mt-3 overflow-hidden rounded-lg border bg-muted/30">
			<div className="flex items-center justify-between border-b px-3 py-1.5 text-xs text-muted-foreground">
				<span className="flex items-center gap-2">
					<Icon
						icon={
							block.kind === "terminal"
								? faRectangleTerminal
								: faFileLines
						}
					/>
					{block.kind === "terminal" ? "Terminal" : block.fileName}
				</span>
				<CopyTrigger value={block.code}>
					<Button
						variant="ghost"
						size="icon-sm"
						aria-label="Copy"
						className="size-6 text-muted-foreground"
					>
						<Icon icon={faCopy} />
					</Button>
				</CopyTrigger>
			</div>
			<div className="overflow-x-auto bg-background py-3 text-xs leading-relaxed [&_pre]:w-max [&_pre]:min-w-full [&_pre]:px-1">
				<CodePreview
					code={block.code}
					language={block.kind === "terminal" ? "bash" : "typescript"}
				/>
			</div>
		</div>
	);
}

function LinkList({
	title,
	links,
	moreUrl,
	mono,
}: {
	title: string;
	links: DocLink[];
	moreUrl: string;
	mono?: boolean;
}) {
	return (
		<section>
			<h3 className="text-sm font-semibold">{title}</h3>
			<ul className="mt-2 flex flex-col">
				{links.map((link) => (
					<li key={link.label}>
						<a
							href={link.url}
							target="_blank"
							rel="noreferrer"
							className={cn(
								"group flex items-center justify-between gap-2 rounded-md py-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground",
								mono && "font-mono text-xs",
							)}
						>
							<span className="truncate">{link.label}</span>
							<Icon
								icon={faArrowUpRightFromSquare}
								className="shrink-0 text-[10px] opacity-0 transition-opacity group-hover:opacity-100"
							/>
						</a>
					</li>
				))}
			</ul>
			<a
				href={moreUrl}
				target="_blank"
				rel="noreferrer"
				className="mt-2 inline-flex items-center gap-1.5 text-xs font-medium text-foreground/80 hover:text-foreground"
			>
				View all
				<Icon icon={faArrowUpRightFromSquare} className="text-[10px]" />
			</a>
		</section>
	);
}
