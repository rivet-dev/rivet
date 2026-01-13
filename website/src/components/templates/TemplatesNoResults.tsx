"use client";

import { templates } from "@/data/templates/shared";
import { useTemplatesFilter } from "./TemplatesFilterContext";

export function TemplatesNoResults() {
	const { isTemplateVisible } = useTemplatesFilter();

	// Check if any templates are visible
	const hasVisibleTemplates = templates.some((template) => isTemplateVisible(template));

	if (hasVisibleTemplates) {
		return null;
	}

	return (
		<div className="text-center py-12 text-zinc-400 col-span-full">
			No templates found matching your filters
		</div>
	);
}
