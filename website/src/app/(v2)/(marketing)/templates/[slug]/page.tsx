import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { templates, TECHNOLOGIES, TAGS, type Template } from "@/data/templates/shared";
import { VanillaMarkdown } from "@/components/VanillaMarkdown";
import { TemplateCard } from "../components/TemplateCard";
import { Code } from "@/components/v2/Code";
import { Icon, faGithub, faVercel, faRailway } from "@rivet-gg/icons";
import Link from "next/link";
import Image from "next/image";
import { CodeBlock } from "@/components/CodeBlock";
import fs from "node:fs/promises";
import path from "node:path";
import { DeployDropdown } from "./DeployDropdown";

interface Props {
	params: Promise<{ slug: string }>;
}

export async function generateStaticParams() {
	return templates.map((template) => ({
		slug: template.name,
	}));
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
	const { slug } = await params;
	const template = templates.find((t) => t.name === slug);

	if (!template) {
		return {
			title: "Template Not Found - Rivet",
		};
	}

	return {
		title: `${template.displayName} - Rivet Templates`,
		description: template.description,
		alternates: {
			canonical: `https://www.rivet.dev/templates/${slug}/`,
		},
	};
}

async function getReadmeContent(templateName: string): Promise<string> {
	try {
		const readmePath = path.join(
			process.cwd(),
			"..",
			"examples",
			templateName,
			"README.md",
		);
		const content = await fs.readFile(readmePath, "utf-8");
		return content;
	} catch (error) {
		console.error(`Failed to read README for ${templateName}:`, error);
		return "# README not found\n\nThe README for this template could not be loaded.";
	}
}

function getRelatedTemplates(template: Template): Template[] {
	// Find related templates based on shared tags
	const relatedTemplates = templates
		.filter((t) => {
			if (t.name === template.name) return false;
			return template.tags.some((tag) => t.tags.includes(tag));
		})
		.slice(0, 3);

	// If no related templates with shared tags, just show any 3 templates
	return relatedTemplates.length > 0
		? relatedTemplates
		: templates.filter((t) => t.name !== template.name).slice(0, 3);
}

function cleanReadmeContent(content: string): string {
	return content
		.replace(/^#\s+.+$/m, '') // Remove first heading
		.replace(/^\n+/, '') // Remove leading newlines
		.replace(/^.+?(?=\n\n|\n#)/s, '') // Remove first paragraph
		.replace(/##\s+Getting Started[\s\S]*?(?=\n##|$)/, '') // Remove Getting Started section
		.replace(/##\s+License[\s\S]*$/, '') // Remove License section
		.trim();
}

export default async function Page({ params }: Props) {
	const { slug } = await params;
	const template = templates.find((t) => t.name === slug);

	if (!template) {
		notFound();
	}

	const readmeContent = await getReadmeContent(template.name);
	const cleanedReadmeContent = cleanReadmeContent(readmeContent);
	const displayedRelated = getRelatedTemplates(template);
	const githubUrl = `https://github.com/rivet-dev/rivet/tree/main/examples/${template.name}`;

	// Strip markdown links from description
	const description = template.description.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');

	// Construct Vercel deploy URL with demo card parameters
	const vercelDeployUrl = new URL('https://vercel.com/new/clone');
	vercelDeployUrl.searchParams.set('repository-url', `https://github.com/rivet-dev/rivet/tree/main/examples/${template.name}`);
	vercelDeployUrl.searchParams.set('project-name', template.displayName);
	vercelDeployUrl.searchParams.set('demo-title', template.displayName);
	vercelDeployUrl.searchParams.set('demo-description', description);
	vercelDeployUrl.searchParams.set('demo-image', `https://www.rivet.dev/examples/${template.name}/image.png`);
	vercelDeployUrl.searchParams.set('demo-url', `https://www.rivet.dev/templates/${template.name}`);

	return (
		<main className="min-h-screen w-full max-w-[1500px] mx-auto md:px-8 font-sans selection:bg-[#FF4500]/30 selection:text-orange-200">
			{/* Header with Image */}
			<div className={`relative w-full pb-12 ${template.noFrontend ? 'pt-48' : 'pt-24'}`}>
				<div className="mx-auto max-w-7xl px-6">
					{!template.noFrontend ? (
						<div className="relative">
							{/* Screenshot on top */}
							<div className="relative">

								{/* Linear gradient overlay - darker on bottom */}
								<div
									className='hidden md:block absolute inset-0 pointer-events-none rounded-xl'
									style={{
										background: 'linear-gradient(to bottom, transparent 0%, transparent 30%, rgba(0, 0, 0, 0.3) 60%, rgb(0, 0, 0) 100%)',
										zIndex: 10
									}}
								/>

								<div
									className="relative overflow-hidden rounded-xl border border-white/10 bg-zinc-900/50 backdrop-blur-xl"
									style={{
										maskImage: 'radial-gradient(ellipse 300% 100% at 50% 90%, rgba(0,0,0,0.3) 0%, rgba(0,0,0,0.6) 15%, rgba(0,0,0,0.9) 25%, black 35%, black 50%)',
										WebkitMaskImage: 'radial-gradient(ellipse 300% 100% at 50% 90%, rgba(0,0,0,0.3) 0%, rgba(0,0,0,0.6) 15%, rgba(0,0,0,0.9) 25%, black 35%, black 50%)'
									}}
								>
									{/* Top Shine Highlight */}
									<div className="absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent" />
									{/* Window Bar */}
									<div className="flex items-center gap-2 border-b border-white/5 bg-white/5 px-4 py-3">
										<div className="flex gap-1.5">
											<div className="h-3 w-3 rounded-full border border-zinc-500/50 bg-zinc-500/20" />
											<div className="h-3 w-3 rounded-full border border-zinc-500/50 bg-zinc-500/20" />
											<div className="h-3 w-3 rounded-full border border-zinc-500/50 bg-zinc-500/20" />
										</div>
									</div>
									{/* Content Area - Screenshot */}
									<div className="relative w-full bg-zinc-900/50 aspect-video">
										<Image
											src={`/examples/${template.name}/image.png`}
											alt={template.displayName}
											className="h-full w-full object-cover"
											fill
											sizes="(max-width: 1024px) 100vw, 80vw"
											priority
											quality={90}
										/>
									</div>
								</div>
							</div>

							{/* Text content overlapping bottom */}
							<div className="relative mt-8 px-6 md:-mt-24 z-20">
								<div className="mx-auto max-w-4xl text-center">
									<h1 className="mb-6 text-3xl font-medium tracking-tight text-white md:text-5xl">
										{template.displayName}
									</h1>
									<p className="mb-8 mx-auto max-w-2xl text-lg leading-relaxed text-zinc-400">
										{description}
									</p>
								</div>
							</div>
						</div>
					) : (
						<div className="text-center">
							<h1 className="text-5xl font-medium text-white mb-4">
								{template.displayName}
							</h1>
							<p className="text-xl text-zinc-400 max-w-2xl mx-auto">
								{description}
							</p>
						</div>
					)}
				</div>
			</div>

			{/* Content Section */}
			<div className="relative mx-auto max-w-2xl px-6 lg:max-w-[1400px] lg:px-8 py-16">
				<div className="flex flex-col lg:flex-row gap-12">
					{/* Left Column - README Content */}
					<div className="flex-1 lg:w-2/3">
						<article className="prose prose-invert prose-zinc max-w-none">
							<VanillaMarkdown>{cleanedReadmeContent}</VanillaMarkdown>
						</article>
					</div>

					{/* Right Column - Sidebar */}
					<aside className="lg:w-1/3 lg:max-w-sm">
						<div className="space-y-6">
						<div className="rounded-xl bg-neutral-950 border border-white/20 p-6 pt-5">
								<h3 className="text-sm font-medium text-white mb-4">
									Get Started
								</h3>
								<div className="space-y-2">
									<Link
										href={githubUrl}
										target="_blank"
										rel="noopener noreferrer"
										className="flex items-center gap-2 w-full rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white hover:border-white/20 transition-colors"
									>
										<Icon icon={faGithub} className="text-sm" />
										View on GitHub
									</Link>
									{/*<Link
										href={vercelDeployUrl.toString()}
										target="_blank"
										rel="noopener noreferrer"
										className="flex items-center gap-2 w-full rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white hover:border-white/20 transition-colors"
									>
										<Icon icon={faVercel} className="text-sm" />
										Deploy to Vercel
									</Link>
									<Link
										href={`https://railway.app/new/template?template=https://github.com/rivet-dev/rivet/tree/main/examples/${template.name}`}
										target="_blank"
										rel="noopener noreferrer"
										className="flex items-center gap-2 w-full rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white hover:border-white/20 transition-colors"
									>
										<Icon icon={faRailway} className="text-sm" />
										Deploy to Railway
									</Link>
									<DeployDropdown />*/}
								</div>
							</div>

							{/* Setup Commands */}
							<div className="rounded-xl bg-neutral-950 border border-white/20 overflow-hidden">
								<h3 className="text-sm font-medium text-white px-6 py-4 border-b border-white/10">
									Run Locally
								</h3>
								<div className="[&>*]:mb-0">
									<Code language="bash" flush>
										<CodeBlock
											lang="bash"
											className="px-4"
											code={`git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/${template.name}
npm install
npm run dev`}
										>
										</CodeBlock>
									</Code>
								</div>
							</div>

							{/* Technologies */}
							<div className="rounded-xl bg-neutral-950 border border-white/20 p-6 pt-5">
								<h3 className="text-sm font-medium text-white mb-4">
									Technologies
								</h3>
								<div className="flex flex-wrap gap-2">
									{template.technologies.map((tech) => {
										const techInfo = TECHNOLOGIES.find((t) => t.name === tech);
										return (
											<span
												key={tech}
												className="inline-flex items-center px-3 py-1.5 rounded-md text-sm font-medium bg-white/5 text-zinc-300 border border-white/10"
											>
												{techInfo?.displayName || tech}
											</span>
										);
									})}
								</div>
							</div>

							{/* Tags */}
							{template.tags.length > 0 && (
								<div className="rounded-xl bg-neutral-950 border border-white/20 p-6 pt-5">
									<h3 className="text-sm font-medium text-white mb-4">Tags</h3>
									<div className="flex flex-wrap gap-2">
										{template.tags.map((tag) => {
											const tagInfo = TAGS.find((t) => t.name === tag);
											return (
												<span
													key={tag}
													className="inline-flex items-center px-3 py-1.5 rounded-md text-sm font-medium bg-white/5 text-zinc-300 border border-white/10"
												>
													{tagInfo?.displayName || tag}
												</span>
											);
										})}
									</div>
								</div>
							)}
						</div>
					</aside>
				</div>

				{/* Related Templates Section */}
				<div className="mt-24 border-t border-white/10 pt-16">
					<h2 className="text-3xl font-medium text-white mb-8">
						Related Templates
					</h2>
					<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
						{displayedRelated.map((relatedTemplate) => (
							<TemplateCard
								key={relatedTemplate.name}
								template={relatedTemplate}
							/>
						))}
					</div>
				</div>
			</div>
		</main>
	);
}
