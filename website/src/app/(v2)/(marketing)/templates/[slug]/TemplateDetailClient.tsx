"use client";

import type { Template } from "@/data/templates/shared";
import { templates, TECHNOLOGIES, TAGS } from "@/data/templates/shared";
import { Markdown } from "@/components/Markdown";
import { TemplateCard } from "../components/TemplateCard";
import { Icon, faGithub } from "@rivet-gg/icons";
import Link from "next/link";

interface TemplateDetailClientProps {
	template: Template;
	readmeContent: string;
}

export default function TemplateDetailClient({
	template,
	readmeContent,
}: TemplateDetailClientProps) {
	// Find related templates based on shared tags
	const relatedTemplates = templates
		.filter((t) => {
			// Exclude the current template
			if (t.name === template.name) return false;

			// Find templates with at least one shared tag
			return template.tags.some((tag) => t.tags.includes(tag));
		})
		.slice(0, 3);

	// If no related templates with shared tags, just show any 3 templates
	const displayedRelated =
		relatedTemplates.length > 0
			? relatedTemplates
			: templates.filter((t) => t.name !== template.name).slice(0, 3);

	const githubUrl = `https://github.com/rivet-dev/rivetkit/tree/main/examples/${template.name}`;

	return (
		<main className="min-h-screen w-full max-w-[1500px] mx-auto md:px-8 font-sans selection:bg-[#FF4500]/30 selection:text-orange-200">
			{/* Header with Image */}
			<div className="relative w-full pt-24 pb-12">
				<div className="mx-auto max-w-[1400px] px-6 lg:px-8">
					{/* Placeholder Image */}
					<div className="w-full h-[400px] bg-gradient-to-br from-zinc-800 to-zinc-900 rounded-xl flex items-center justify-center text-zinc-600 mb-8">
						<svg
							className="w-24 h-24"
							fill="none"
							stroke="currentColor"
							viewBox="0 0 24 24"
						>
							<path
								strokeLinecap="round"
								strokeLinejoin="round"
								strokeWidth={1.5}
								d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
							/>
						</svg>
					</div>

					{/* Title and Description */}
					<div className="text-center">
						<h1 className="text-5xl font-medium text-white mb-4">
							{template.displayName}
						</h1>
						<p className="text-xl text-zinc-400 max-w-2xl mx-auto">
							{template.description}
						</p>
					</div>
				</div>
			</div>

			{/* Content Section */}
			<div className="relative mx-auto max-w-2xl px-6 lg:max-w-[1400px] lg:px-8 py-16">
				<div className="flex flex-col lg:flex-row gap-12">
					{/* Left Column - README Content */}
					<div className="flex-1 lg:w-2/3">
						<article className="prose prose-invert prose-zinc max-w-none">
							<Markdown>{readmeContent}</Markdown>
						</article>
					</div>

					{/* Right Column - Sidebar */}
					<aside className="lg:w-1/3 lg:max-w-sm">
						<div className="space-y-6">
							{/* GitHub Link */}
							<div className="rounded-xl bg-white/2 border border-white/20 p-6">
								<h3 className="text-sm font-medium text-white mb-4">
									Repository
								</h3>
								<Link
									href={githubUrl}
									target="_blank"
									rel="noopener noreferrer"
									className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-white/5 hover:bg-white/10 border border-white/10 hover:border-white/20 text-white transition-all w-full justify-center"
								>
									<Icon icon={faGithub} className="text-lg" />
									<span className="text-sm font-medium">View on GitHub</span>
								</Link>
							</div>

							{/* Technologies */}
							<div className="rounded-xl bg-white/2 border border-white/20 p-6">
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
								<div className="rounded-xl bg-white/2 border border-white/20 p-6">
									<h3 className="text-sm font-medium text-white mb-4">Tags</h3>
									<div className="flex flex-wrap gap-2">
										{template.tags.map((tag) => {
											const tagInfo = TAGS.find((t) => t.name === tag);
											return (
												<span
													key={tag}
													className="inline-flex items-center px-3 py-1.5 rounded-md text-sm font-medium bg-[#FF4500]/10 text-[#FF4500] border border-[#FF4500]/20"
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
