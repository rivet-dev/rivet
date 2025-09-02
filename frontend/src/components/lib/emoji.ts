const U200D = String.fromCharCode(0x200d);
const UFE0Fg = /\uFE0F/g;

/**
 * @see [Twemoji Repository](https://github.com/twitter/twemoji/blob/d94f4cf793e6d5ca592aa00f58a88f6a4229ad43/scripts/build.js#L344-L350)
 */

export function convertEmojiToUriFriendlyString(rawText: string) {
	return toCodePoint(
		rawText.indexOf(U200D) < 0 ? rawText.replace(UFE0Fg, "") : rawText,
	);
}

/**
 *
 * @see [Twemoji Repository](https://github.com/twitter/twemoji/blob/d94f4cf793e6d5ca592aa00f58a88f6a4229ad43/scripts/build.js#L571-L589)
 */
function toCodePoint(unicodeSurrogates: string, sep?: string) {
	/* eslint-disable */
	const r = [];
	let c = 0;
	let p = 0;
	let i = 0;

	while (i < unicodeSurrogates.length) {
		c = unicodeSurrogates.charCodeAt(i++);
		if (p) {
			r.push(
				(0x10000 + ((p - 0xd800) << 10) + (c - 0xdc00)).toString(16),
			);
			p = 0;
		} else if (0xd800 <= c && c <= 0xdbff) {
			p = c;
		} else {
			r.push(c.toString(16));
		}
	}

	return r.join(sep || "-");
	/* eslint-enable */
}
