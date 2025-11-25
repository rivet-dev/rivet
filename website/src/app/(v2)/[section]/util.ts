export const VALID_SECTIONS = ["docs", "guides"];

export function buildPathComponents(
	section: string,
	page?: string[],
): string[] {
	let defaultedPage = page ?? [];

	if (defaultedPage[defaultedPage.length - 1] === "index") {
		defaultedPage = defaultedPage.slice(0, -1);
	}

	return [section, ...defaultedPage];
}

export function buildFullPath(pathComponents: string[]): string {
	return `/${pathComponents.join("/")}`;
}

