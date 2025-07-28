/**
 * This file is a proxy for the MDX files in the docs directory.
 * It loads the MDX file based on the slug and renders it.
 * It also generates the metadata for the page.
 * We avoid using the new `page.mdx` convention because its harder to navigate the docs when editing.
 * Also, importing the MDX files directly allow us to use other exports from the MDX files.
 */

import fs from "node:fs/promises";
import path from "node:path";
import { DocsNavigation } from "@/components/DocsNavigation";
import { DocsTableOfContents } from "@/components/DocsTableOfContents";
import { DocsPageDropdown } from "@/components/DocsPageDropdown";
import { Prose } from "@/components/Prose";
import { type Sitemap, findActiveTab } from "@/lib/sitemap";
import { sitemap } from "@/sitemap/mod";
import { Button } from "@rivet-gg/components";
import { Icon, faPencil } from "@rivet-gg/icons";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { VALID_SECTIONS, buildFullPath, buildPathComponents } from "./util";
import { Comments } from "@/components/Comments";
import clsx from "clsx";

interface Param {
	section: string;
	page?: string[];
}

function createParamsForFile(section, file): Param {
	const step1 = file.replace("index.mdx", "");
	const step2 = step1.replace(".mdx", "");
	const step3 = step2.split("/");
	const step4 = step3.filter((x) => x.length > 0);

	return {
		section,
		page: step4,
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
					throw new Error(
						`Content not found for path: ${path.join("/")}`,
					);
				}
				throw indexError;
			}
		}
		throw error;
	}
}

export async function generateMetadata({
	params: { section, page },
}): Promise<Metadata> {
	const path = buildPathComponents(section, page);
	const {
		component: { title, description },
	} = await loadContent(path);

	return {
		title: `${title} - Rivet`,
		description,
	};
}

export default async function CatchAllCorePage({ params: { section, page } }) {
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
						<div className="relative">
							<div className="absolute top-5 right-0">
								<DocsPageDropdown
									title={title || "Documentation"}
									markdownPath={markdownPath}
									currentUrl={fullPath}
								/>
							</div>
						</div>
						<Prose
							as="article"
							className="max-w-prose lg:max-w-prose mx-auto"
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
								href={`https://github.com/rivet-gg/rivet/edit/main/site/src/content/${componentSourcePath}`}
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

export async function generateStaticParams() {
	const staticParams: Param[] = [];

	for (const section of VALID_SECTIONS) {
		const dir = path.join(process.cwd(), "src", "content", section);

		const dirs = await fs.readdir(dir, { recursive: true });
		const files = dirs.filter((file) => file.endsWith(".mdx"));

		const sectionParams = files.map((file) => {
			const param = createParamsForFile(section, file);
			return param;
		});

		staticParams.push(...sectionParams);
	}

	return staticParams;
}
