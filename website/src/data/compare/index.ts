import type { CompareEntry } from './types';
import { cloudflareDurableObjects } from './cloudflare-durable-objects';
import { temporal } from './temporal';

// Ordered registry of comparison pages. Adding a page means adding one data
// module in this directory and one line here; the /compare routes and the
// "Other comparisons" links derive from this list.
export const compareEntries: CompareEntry[] = [cloudflareDurableObjects, temporal];

export function getCompareEntry(slug: string): CompareEntry | undefined {
	return compareEntries.find((entry) => entry.slug === slug);
}
