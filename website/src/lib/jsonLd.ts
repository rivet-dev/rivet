// Serialize a schema object for injection via set:html inside an inline
// <script type="application/ld+json"> element. Escapes < so content-derived
// strings can never terminate the script element early.
export function jsonLdString(schema: unknown): string {
	return JSON.stringify(schema).replace(/</g, '\\u003c');
}
