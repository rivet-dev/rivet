"use client";

import type { Template } from "@/data/templates/shared";
import type { ReactNode } from "react";
import { useTemplatesFilter } from "./TemplatesFilterContext";

interface TemplateCardWrapperProps {
	template: Template;
	children: ReactNode;
}

export function TemplateCardWrapper({ template, children }: TemplateCardWrapperProps) {
	const { isTemplateVisible } = useTemplatesFilter();
	const visible = isTemplateVisible(template);

	return (
		<div style={{ display: visible ? "block" : "none" }}>
			{children}
		</div>
	);
}
