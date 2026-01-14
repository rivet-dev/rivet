import type { Template } from "@/data/templates/shared";

interface TemplateImageProps {
	template: Template;
}

export function TemplateImage({
	template,
}: TemplateImageProps) {
	if (template.noFrontend) {
		return (
			<div className="absolute inset-0 flex items-center justify-center bg-gradient-to-br from-zinc-800 to-zinc-900">
				<span className="text-zinc-500 text-sm font-medium">No Frontend</span>
			</div>
		);
	}

	return (
		<img src={`/examples/${template.name}/image.png`}
			alt={template.displayName}
			className="object-cover absolute inset-0 w-full h-full"
		/>
	);
}
