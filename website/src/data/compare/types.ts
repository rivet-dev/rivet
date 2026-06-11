import type { ReactNode } from 'react';
import type { IconProp } from '@rivet-gg/icons';
import type { FaqItem } from '@/data/faqs/types';

// Shared types for the /compare/* pages. Each competitor gets one data module
// in this directory plus a line in the registry in index.ts; the dynamic route
// at src/pages/compare/[slug].astro and the ComparePage island both read from
// the registry.

export type ComparisonStatus = 'yes' | 'no' | 'partial' | 'coming-soon';

export interface FeatureCell {
	status: ComparisonStatus;
	// text may contain JSX, for example links to docs or GitHub. Because of
	// this the data modules are not serializable across the island boundary;
	// the route passes only the slug and the island looks the entry up itself.
	text: ReactNode;
}

export interface FeatureRow {
	feature: string;
	rivet: FeatureCell;
	competitor: FeatureCell;
	importance: string;
}

export interface FeatureGroup {
	title: string;
	rows: FeatureRow[];
}

export interface ChoicePoint {
	title: string;
	description: string;
}

export interface CompareEntry {
	slug: string;
	competitorName: string;
	rivetProductName: string;
	competitorIcon?: IconProp;
	// Page title, for example "Rivet vs X". BaseLayout auto-suffixes " - Rivet".
	title: string;
	// Meta description, 150 to 160 characters.
	description: string;
	heroSubtitle: string;
	rivetSummary: string;
	competitorSummary: string;
	// Both choice lists should be honest; aim for at least 3 points on the
	// competitor side.
	whenToChooseRivet: ChoicePoint[];
	whenToChooseCompetitor: ChoicePoint[];
	featureGroups: FeatureGroup[];
	// Verdict paragraphs rendered in order.
	verdict: string[];
	migration?: { heading: string; body: string };
	// Rendered visibly on the page and emitted as FAQPage JSON-LD by the route.
	faq: FaqItem[];
	// ISO date, rendered in the hero and used as WebPage dateModified.
	lastUpdated: string;
	keywords?: string[];
}
