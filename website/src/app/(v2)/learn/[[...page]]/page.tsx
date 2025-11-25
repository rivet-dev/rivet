import { notFound } from "next/navigation";
import type { Metadata, ResolvingMetadata } from "next";
import fs from "node:fs/promises";
import path from "node:path";
import { Navigation } from "../components/Navigation";
import { ActEnd } from "../components/theatrical";
import { DocsTableOfContents } from "@/components/DocsTableOfContents";

interface PageProps {
	params: Promise<{
		page?: string[];
	}>;
}

const contentDir = "src/content/learn";

async function loadContent(path: string[]) {
	const module = path.join("/");

	try {
		return {
			path: `${module}.mdx`,
			component: await import(`@/content/learn/${module}.mdx`),
		};
	} catch (error) {
		const indexModule = `${module}/index`;
		try {
			return {
				path: `${indexModule}.mdx`,
				component: await import(`@/content/learn/${indexModule}.mdx`),
			};
		} catch {
			return null;
		}
	}
}

export async function generateStaticParams(): Promise<{ page: string[] }[]> {
	const dir = path.join(process.cwd(), contentDir);
	const files = await fs.readdir(dir, { recursive: true });
	const mdxFiles = files.filter((file) => file.endsWith(".mdx"));

	return mdxFiles.map((file) => {
		const step1 = file.replace("index.mdx", "");
		const step2 = step1.replace(".mdx", "");
		const step3 = step2.split("/");
		const segments = step3.filter((x) => x.length > 0);
		return { page: segments };
	});
}

export async function generateMetadata(
	{ params }: PageProps,
	parent: ResolvingMetadata,
): Promise<Metadata> {
	const resolvedParams = await params;
	const page = resolvedParams.page ?? [];
	const content = await loadContent(page);

	if (!content) {
		return {};
	}

	const { component } = content;
	const title = component.title || (await parent).title?.absolute || "Learn";
	const description =
		component.description || (await parent).description || "";

	return {
		title,
		description,
	};
}

export default async function Page({ params }: PageProps) {
	const resolvedParams = await params;
	const page = resolvedParams.page ?? ["index"];
	const content = await loadContent(page);

	if (!content) {
		notFound();
	}

	const { component: Content } = content;
	const act = Content.act || "";
	const subtitle = Content.subtitle || "";
	const tableOfContents = Content.tableOfContents || [];

	// Determine if we're on the index page
	const isIndexPage = page.length === 1 && page[0] === "index";

	// Extract scene number from path (e.g., "scene-1-a-radically-simpler-architecture" -> "Scene 1")
	const getSceneLabel = () => {
		if (page.length < 2) return "";
		const scenePart = page[page.length - 1];
		const sceneMatch = scenePart?.match(/scene-(\d+)/);
		return sceneMatch ? `Scene ${sceneMatch[1]}` : "";
	};
	const scene = getSceneLabel();

	return (
		<>
			<div className={`px-6 py-16 md:py-24 pb-32 animate-in slide-in-from-bottom-4 duration-700 ${isIndexPage ? 'max-w-2xl mx-auto' : 'relative'}`}>
				{!isIndexPage ? (
					<>
						<main className="max-w-3xl mx-auto">
							{/* Act Header */}
							{act && (
								<header className="text-center mb-20 relative">
									<span className="font-serif italic text-[#a8a29e] text-base md:text-lg block mb-4 opacity-80">
										{act}{scene && `, ${scene}`}
									</span>
									{Content.title && (
										<h1 className="font-display text-4xl md:text-6xl text-[#e7e5e4] mb-6 tracking-wide uppercase">
											{Content.title}
										</h1>
									)}
									{subtitle && (
										<p className="font-serif text-[#a8a29e] text-lg md:text-xl italic mb-8">
											{subtitle}
										</p>
									)}
									<div className="w-32 h-px bg-[#d4b483] mx-auto opacity-50" />
								</header>
							)}

							{/* Content with theatrical MDX components */}
							<article className="prose prose-invert prose-lg md:prose-xl prose-p:font-serif prose-headings:font-display prose-headings:text-[#e7e5e4] prose-ul:list-none prose-ul:pl-6 prose-li:font-serif prose-li:before:content-['◆'] prose-li:before:text-[#d4b483] prose-li:before:mr-3 prose-li:before:opacity-60 max-w-none">
								<Content.default />
							</article>

							{/* Auto-generate act end marker for scene pages */}
							{scene && <ActEnd scene={scene} />}
						</main>

						{/* Table of Contents - positioned on the left edge */}
						<aside className="hidden xl:block fixed top-32 left-16 w-64 opacity-50 hover:opacity-100 transition-opacity duration-200">
							<div className="font-serif text-sm">
								<h2 className="font-display text-[#a8a29e] text-xs uppercase tracking-wider mb-2 opacity-60">
									On This Page
								</h2>
								<DocsTableOfContents
									tableOfContents={tableOfContents}
									showNewsletter={false}
									className="!pt-0 [&_a]:font-serif [&_a]:text-[#a8a29e] [&_a:hover]:text-[#e7e5e4] [&_a[aria-current=page]]:text-[#d4b483] [&_a]:px-0 [&_a]:pl-0"
								/>
							</div>
						</aside>
					</>
				) : (
					<main>
						{/* Act Header */}
						{act && (
							<header className="text-center mb-20 relative">
								<span className="font-serif italic text-[#a8a29e] text-base md:text-lg block mb-4 opacity-80">
									{act}{scene && `, ${scene}`}
								</span>
								{Content.title && (
									<h1 className="font-display text-4xl md:text-6xl text-[#e7e5e4] mb-6 tracking-wide uppercase">
										{Content.title}
									</h1>
								)}
								{subtitle && (
									<p className="font-serif text-[#a8a29e] text-lg md:text-xl italic mb-8">
										{subtitle}
									</p>
								)}
								<div className="w-32 h-px bg-[#d4b483] mx-auto opacity-50" />
							</header>
						)}

						{/* Content with theatrical MDX components */}
						<article className="prose prose-invert prose-lg md:prose-xl prose-p:font-serif prose-headings:font-display prose-headings:text-[#e7e5e4] prose-ul:list-none prose-ul:pl-6 prose-li:font-serif prose-li:before:content-['◆'] prose-li:before:text-[#d4b483] prose-li:before:mr-3 prose-li:before:opacity-60 max-w-none">
							<Content.default />
						</article>
					</main>
				)}
			</div>

			{/* Only show navigation on content pages, not on index */}
			{!isIndexPage && (
				<Navigation
					showPrev={false}
					showNext={false}
				/>
			)}
		</>
	);
}
