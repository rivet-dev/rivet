// export const VALID_SECTIONS = ['docs', 'use-cases', 'examples', 'compare'];
export const VALID_SECTIONS = ["docs", "compare"];

export function buildPathComponents(
	section: string,
	page?: string[],
): string[] {
	// Add default page
	let defaultedPage = page ?? [];

	// Remove index suffix
	if (defaultedPage[defaultedPage.length - 1] === "index") {
		defaultedPage = defaultedPage.slice(0, -1);
	}

	return [section, ...defaultedPage];
}

export function buildFullPath(pathComponents: string[]): string {
	return `/${pathComponents.join("/")}`;
}
