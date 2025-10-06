"use client";

import { useAutofillStore } from "@/stores/autofill-store";
import { useMemo } from "react";

export function AutofillFooter() {
	const { projects, namespaces, selectedProjectId, selectedNamespaceId } =
		useAutofillStore();

	const selectedProject = useMemo(
		() => projects.find((p) => p.id === selectedProjectId),
		[projects, selectedProjectId],
	);

	const selectedNamespace = useMemo(
		() => namespaces.find((n) => n.id === selectedNamespaceId),
		[namespaces, selectedNamespaceId],
	);

	if (!selectedProject || !selectedNamespace) {
		return null;
	}

	return (
		<div className="text-muted-foreground flex items-center gap-1 text-xs">
			<span className="opacity-60">Using:</span>
			<span className="font-medium">
				{selectedProject.organizationSlug} / {selectedProject.slug} /{" "}
				{selectedNamespace.slug}
			</span>
		</div>
	);
}
