import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { templates } from "@/data/templates/shared";
import fs from "node:fs/promises";
import path from "node:path";
import TemplateDetailClient from "./TemplateDetailClient";

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

export default async function Page({ params }: Props) {
	const { slug } = await params;
	const template = templates.find((t) => t.name === slug);

	if (!template) {
		notFound();
	}

	const readmeContent = await getReadmeContent(template.name);

	return (
		<TemplateDetailClient template={template} readmeContent={readmeContent} />
	);
}
