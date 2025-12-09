import type { Template } from "@/data/templates/shared";
import Image from "next/image";

interface TemplateImageProps {
	template: Template;
	priority?: boolean;
	sizes?: string;
}

export function TemplateImage({
	template,
	priority = false,
	sizes = "(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw",
}: TemplateImageProps) {
	if (template.noFrontend) {
		return (
			<div className="absolute inset-0 flex items-center justify-center bg-gradient-to-br from-zinc-800 to-zinc-900">
				<span className="text-zinc-500 text-sm font-medium">No Frontend</span>
			</div>
		);
	}

	return (
		<Image
			src={`/examples/${template.name}/image.png`}
			alt={template.displayName}
			fill
			className="object-cover"
			sizes={sizes}
			priority={priority}
		/>
	);
}
