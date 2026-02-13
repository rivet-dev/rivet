export function getContentSlugPath(entryId: string): string {
	return entryId === 'index' ? '' : entryId.replace(/\/index$/, '');
}

export function getContentParamSlug(entryId: string): string | undefined {
	const slugPath = getContentSlugPath(entryId);
	return slugPath || undefined;
}
