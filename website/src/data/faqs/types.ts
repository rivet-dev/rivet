// Shared FAQ data types. Each page's FAQ content lives in one data module
// under src/data/faqs/ and feeds both the visible FaqSection rendering and
// the FAQPage JSON-LD emitted by FaqJsonLd.astro, so the schema can never
// diverge from what is shown on the page.

export interface FaqItem {
	question: string;
	// answerHtml is plain text or HTML limited to the tags Google allows in
	// FAQPage Answer.text: h1-h6, br, ol, ul, li, a, p, div, b, strong, i, em.
	// Links must use absolute https://rivet.dev URLs.
	answerHtml: string;
}

// Builds a schema.org FAQPage object from FAQ items. Emit it only through
// FaqJsonLd.astro so each URL gets at most one FAQPage block.
export function faqPageSchema(items: FaqItem[]) {
	return {
		"@context": "https://schema.org",
		"@type": "FAQPage",
		mainEntity: items.map((item) => ({
			"@type": "Question",
			name: item.question,
			acceptedAnswer: {
				"@type": "Answer",
				text: item.answerHtml,
			},
		})),
	};
}
