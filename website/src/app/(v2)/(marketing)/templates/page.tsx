import type { Metadata } from "next";
import { templates, type Technology, type Tag } from "@/data/templates/shared";
import { TemplateCard } from "./components/TemplateCard";
import { TemplatesSidebar } from "./components/TemplatesSidebar";
import { TemplatesFilterProvider } from "./components/TemplatesFilterContext";
import { TemplatesSearch } from "./components/TemplatesSearch";
import { TemplateCardWrapper } from "./components/TemplateCardWrapper";
import { TemplatesNoResults } from "./components/TemplatesNoResults";

export const metadata: Metadata = {
	title: "Templates - Rivet",
	description:
		"Explore Rivet templates and examples to quickly start building with Rivet Actors. Find templates for AI agents, real-time apps, games, and more.",
	alternates: {
		canonical: "https://www.rivet.dev/templates/",
	},
};

// Get unique tags from all templates
function getAllTags(): Tag[] {
	const tagsSet = new Set<Tag>();
	templates.forEach((template) => {
		template.tags.forEach((tag) => tagsSet.add(tag as Tag));
	});
	return Array.from(tagsSet).sort();
}

// Get unique technologies from all templates
function getAllTechnologies(): Technology[] {
	const techSet = new Set<Technology>();
	templates.forEach((template) => {
		template.technologies.forEach((tech) => techSet.add(tech as Technology));
	});
	return Array.from(techSet).sort();
}

export default function Page() {
	const allTags = getAllTags();
	const allTechnologies = getAllTechnologies();

	return (
		<TemplatesFilterProvider templates={templates}>
			<main className="min-h-screen w-full max-w-[1500px] mx-auto md:px-8 font-sans selection:bg-white/20 selection:text-white">
				<div className="relative isolate overflow-hidden pb-8 sm:pb-10 pt-48">
					<div className="mx-auto max-w-[1200px] px-6 lg:px-8">
						<div className="text-center">
							<h1
								className="text-5xl font-medium leading-[1.1] tracking-tighter md:text-7xl"
								style={{ color: "#FAFAFA" }}
							>
								Templates
							</h1>
							<p
								className="mt-6 max-w-2xl mx-auto text-lg md:text-xl leading-[1.2]"
								style={{ color: "#A0A0A0" }}
							>
								Explore Rivet templates and examples to quickly start building
								with Rivet Actors
							</p>
							<div className="mt-8 max-w-md mx-auto">
								<TemplatesSearch />
							</div>
						</div>
					</div>
				</div>

				<div className="relative mx-auto max-w-2xl px-6 lg:max-w-[1400px] lg:px-8 pb-24">
					<div className="flex flex-col lg:flex-row gap-8">
						{/* Sidebar */}
						<TemplatesSidebar
							allTags={allTags}
							allTechnologies={allTechnologies}
						/>

						{/* Main Grid */}
						<div className="flex-1">
							<div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6">
								<TemplatesNoResults />
								{templates.map((template) => (
									<TemplateCardWrapper key={template.name} template={template}>
										<TemplateCard template={template} />
									</TemplateCardWrapper>
								))}
							</div>
						</div>
					</div>
				</div>
			</main>
		</TemplatesFilterProvider>
	);
}
