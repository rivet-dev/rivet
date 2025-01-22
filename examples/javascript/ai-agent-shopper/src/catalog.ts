import CATALOG_ITEMS from "./catalog_items";

export interface CatalogItem {
	slug: string;
	name: string;
	price: number; // cents
}

export function searchCatalogByKeywords(keywords: string[]): CatalogItem[] {
	if (!keywords.length) return [];

	const normalizedTags = keywords.map((tag) => tag.toLowerCase());

	return CATALOG_ITEMS.filter((item) => {
		return normalizedTags.some(
			(tag) =>
				item.slug.toLowerCase().includes(tag) ||
				item.name.toLowerCase().includes(tag),
		);
	});
}

export function getCatalogItemBySlug(slug: string): CatalogItem | undefined {
	return CATALOG_ITEMS.find((item) => item.slug === slug);
}
