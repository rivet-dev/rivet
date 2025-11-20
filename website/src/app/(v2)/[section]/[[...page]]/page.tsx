/**
 * This file is a proxy for the MDX files in the docs directory.
 * It loads the MDX file based on the slug and renders it.
 * It also generates the metadata for the page.
 * We avoid using the new `page.mdx` convention because its harder to navigate the docs when editing.
 * Also, importing the MDX files directly allow us to use other exports from the MDX files.
 */

import fs from "node:fs/promises";
import path from "node:path";
import { Button } from "@rivet-gg/components";
import { faPencil, Icon } from "@rivet-gg/icons";
import clsx from "clsx";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Comments } from "@/components/Comments";
import { DocsNavigation } from "@/components/DocsNavigation";
import { DocsPageDropdown } from "@/components/DocsPageDropdown";
import { DocsTableOfContents } from "@/components/DocsTableOfContents";
import { Prose } from "@/components/Prose";
import { findActiveTab, type Sitemap } from "@/lib/sitemap";
import { sitemap } from "@/sitemap/mod";
import { buildFullPath, buildPathComponents, VALID_SECTIONS } from "./util";

interface Param {
	section: string;
	page?: string[];
}

// Ensure Next.js knows this is a dynamic route
export const dynamicParams = false;

function createParamsForFile(section, file): Param {
	const step1 = file.replace("index.mdx", "");
	const step2 = step1.replace(".mdx", "");
	const step3 = step2.split("/");
	const step4 = step3.filter((x) => x.length > 0);

	return {
		section,
		page: step4.length > 0 ? step4 : undefined,
	};
}

async function loadContent(path: string[]) {
	const module = path.join("/");
	try {
		return {
			path: `${module}.mdx`,
			component: await import(`@/content/${module}.mdx`),
		};
	} catch (error) {
		if (error.code === "MODULE_NOT_FOUND") {
			try {
				const indexModule = `${module}/index`;
				return {
					path: `${indexModule}.mdx`,
					component: await import(`@/content/${indexModule}.mdx`),
				};
			} catch (indexError) {
				if (indexError.code === "MODULE_NOT_FOUND") {
					return notFound();
				}
				throw indexError;
			}
		}
		throw error;
	}
}

export async function generateMetadata({
	params,
}: {
	params: { section: string; page?: string[] };
}): Promise<Metadata> {
	const { section, page } = params;
	const path = buildPathComponents(section, page);
	const {
		component: { title, description },
	} = await loadContent(path);

	const fullPath = buildFullPath(path);
	const canonicalUrl = `https://www.rivet.dev${fullPath}/`;

	return {
		title: `${title} - Rivet`,
		description,
		alternates: {
			canonical: canonicalUrl,
		},
	};
}

export async function generateStaticParams() {
	const staticParams: Array<{ section: string; page?: string[] }> = [];
	const seenParams = new Set<string>();

	for (const section of VALID_SECTIONS) {
		const dir = path.join(process.cwd(), "src", "content", section);

		try {
			// Always add base case first (section root with no page segments)
			// For optional catch-all, omit page property when undefined
			const baseKey = `${section}`;
			if (!seenParams.has(baseKey)) {
				seenParams.add(baseKey);
				staticParams.push({ section });
			}

			// Read all MDX files recursively
			const dirs = await fs.readdir(dir, { recursive: true });
			const files = dirs.filter((file) => file.endsWith(".mdx"));

			for (const file of files) {
				const param = createParamsForFile(section, file);
				
				// For optional catch-all routes, omit page when undefined
				const finalParam: { section: string; page?: string[] } = param.page === undefined 
					? { section: param.section }
					: { section: param.section, page: param.page };

				// Create unique key for deduplication
				const key = finalParam.page 
					? `${finalParam.section}/${finalParam.page.join("/")}`
					: finalParam.section;

				if (!seenParams.has(key)) {
					seenParams.add(key);
					staticParams.push(finalParam);
				}
			}
		} catch (error) {
			// If directory doesn't exist, still add base case
			const baseKey = `${section}`;
			if (!seenParams.has(baseKey)) {
				seenParams.add(baseKey);
				staticParams.push({ section });
			}
		}
	}

	return staticParams;
}

export default async function CatchAllCorePage({
	params: { section, page },
}: {
	params: { section: string; page?: string[] };
}) {
	if (!VALID_SECTIONS.includes(section)) {
		return notFound();
	}

	const path = buildPathComponents(section, page);
	const {
		path: componentSourcePath,
		component: { default: Content, tableOfContents, title, description },
	} = await loadContent(path);

	const fullPath = buildFullPath(path);
	const foundTab = findActiveTab(fullPath, sitemap as Sitemap);
	const parentPage = foundTab?.page.parent;

	// Create markdown path for the dropdown (remove .mdx extension and handle index files)
	const markdownPath = componentSourcePath
		.replace(/\.mdx$/, "")
		.replace(/\/index$/, "")
		.replace(/\\/g, "/");

	return (
		<>
			<aside className="hidden lg:block border-r">
				{foundTab?.tab.sidebar ? (
					<DocsNavigation sidebar={foundTab.tab.sidebar} />
				) : null}
			</aside>
			<div className="flex justify-center w-full">
				<div className="flex gap-8 max-w-6xl w-full">
					<main className="w-full py-8 px-8 lg:mx-0 mx-auto max-w-prose lg:max-w-none">
						<div className="relative flex justify-end">
							<div
								className={clsx(
									"flex items-end md:absolute md:right-0",
									parentPage ? "md:top-5" : "md:top-0",
								)}
							>
								<DocsPageDropdown
									title={title || "Documentation"}
									markdownPath={markdownPath}
									currentUrl={fullPath}
								/>
							</div>
						</div>
						<Prose
							as="article"
							className="max-w-prose lg:max-w-prose mx-auto [&>h1:first-of-type]:pr-44"
						>
							{parentPage && (
								<div className="eyebrow h-5 text-primary text-sm font-semibold">
									{parentPage.title}
								</div>
							)}
							<Content />
						</Prose>
						<div className="border-t mt-8 mb-2" />
						<Button
							variant="ghost"
							asChild
							startIcon={<Icon icon={faPencil} />}
						>
							<a
								href={`https://github.com/rivet-dev/engine/edit/main/website/src/content/${componentSourcePath}`}
								target="_blank"
								rel="noreferrer"
							>
								Suggest changes to this page
							</a>
						</Button>
						<Comments />
					</main>
					{tableOfContents && (
						<aside className="hidden xl:block w-64 min-w-0 flex-shrink-0 pb-4">
							<DocsTableOfContents
								className="lg:max-h-content"
								tableOfContents={tableOfContents}
							/>
						</aside>
					)}
				</div>
			</div>
		</>
	);
}
