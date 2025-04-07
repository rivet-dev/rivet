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
import { Prose } from "@/components/Prose";
import { type Sitemap, findActiveTab } from "@/lib/sitemap";
import { sitemap } from "@/sitemap/mod";
import { Button } from "@rivet-gg/components";
import { Icon, faPencil } from "@rivet-gg/icons";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { VALID_SECTIONS, buildFullPath, buildPathComponents } from "./util";
import { Comments } from "@/components/Comments";

interface Param {
	section: string;
	page?: string[];
}

function createParamsForFile(section, file): Param {
	return {
		section,
		page: [
			...file
				.replace("index.mdx", "")
				.replace(".mdx", "")
				.split("/")
				.filter((x) => x.length > 0),
		],
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
		notFound();
	}

	const path = buildPathComponents(section, page);
	const {
		path: componentSourcePath,
		component: { default: Content, tableOfContents },
	} = await loadContent(path);

	const fullPath = buildFullPath(path);
	const foundTab = findActiveTab(fullPath, sitemap as Sitemap);
	const parentPage = foundTab?.page.parent;

	return (
		<>
			<aside className="hidden md:block">
				{foundTab?.tab.sidebar ? (
					<DocsNavigation sidebar={foundTab.tab.sidebar} />
				) : null}
			</aside>
			<main className="md:mx-auto mt-8 w-full max-w-prose px-8 pb-8">
				<Prose as="article">
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
			<aside className="-order-1 mx-auto w-full min-w-0 max-w-3xl flex-shrink-0 pb-4 pl-4 md:order-none xl:mx-0">
				<DocsTableOfContents
					className="lg:max-h-content"
					tableOfContents={tableOfContents}
				/>
			</aside>
		</>
	);
}

export async function generateStaticParams() {
	const staticParams: Param[] = [];
	for (const section of VALID_SECTIONS) {
		const dir = path.join(process.cwd(), "src", "content", section);

		const dirs = await fs.readdir(dir, { recursive: true });
		const files = dirs.filter((file) => file.endsWith(".mdx"));

		staticParams.push(
			...files.map((file) => {
				return createParamsForFile(section, file);
			}),
		);
	}

	return staticParams;
}
