export const VALID_SECTIONS = ['docs', 'use-cases', 'examples', 'compare'];

export function buildPathComponents(section: string, page?: string[]): string[] {
    // Add default page
    page = page ?? [];

    // Remove index suffix
    if (page[page.length - 1] == 'index') {
        page = page.slice(0, -1);
    }

    return [section, ...page];
}

export function buildFullPath(pathComponents: string[]): string {
    return `/${pathComponents.join('/')}`;
}