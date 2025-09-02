export type LogFmtValue = boolean | string | null | object;

export const logfmt = {
	/**
	 *  adapts the logfmt parser to the rivet logfmt format
	 *  original: https://github.com/csquared/node-logfmt/blob/6c3c61fcf5b8fdea1bca2ddac60367f616979dfd/lib/logfmt_parser.js#L3
	 */
	parse: (line: string): Record<string, LogFmtValue> => {
		let key = "";
		let value: boolean | string | null = "";
		let inKey = false;
		let inValue = false;
		let inQuote = false;
		let inEscape = false;
		let inJsonLike = false;
		let hadQuote = false;
		const result: Record<string, LogFmtValue> = {};

		for (let i = 0; i <= line.length; i++) {
			const char = line[i];

			if ((char === " " && !inQuote) || i === line.length) {
				if (inKey && key) {
					result[key] = true;
				} else if (inValue) {
					if (value === "true") value = true;
					else if (value === "false") value = false;
					else if (value === "" && !hadQuote) value = null;
					else if (
						value[0] === "{" &&
						value[value.length - 1] === "}"
					) {
						try {
							value = JSON.parse(value);
						} catch {
							// do nothing
						}
					}
					result[key] = value;
					value = "";
				}

				if (i === line.length) break;

				inKey = false;
				inValue = false;
				inQuote = false;
				inEscape = false;
				hadQuote = false;
			} else if (char === "=" && !inQuote) {
				inKey = false;
				inValue = true;
			} else if (char === "\\") {
				inEscape = true;
			} else if (char === "{" && hadQuote && inValue && inQuote) {
				inJsonLike = true;
				value += char;
			} else if (char === "}" && inJsonLike) {
				inJsonLike = false;
				value += char;
			} else if (char === '"' && !inJsonLike) {
				hadQuote = true;
				inQuote = !inQuote;
			} else if (char === "n" && inEscape && inValue && !inJsonLike) {
				value += "\n";
				inEscape = false;
			} else if (char !== " " && !inValue && !inKey) {
				inKey = true;
				key = char;
			} else if (inKey) {
				key += char;
			} else if (inValue) {
				value += char;
			}
		}

		return result;
	},
};
