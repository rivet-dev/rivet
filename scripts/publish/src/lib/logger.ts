/**
 * Prefixed console logger for consistent CI log scanning.
 *
 * Usage:
 *   const log = scoped("npm");
 *   log.info("publishing ...");  ->  [npm] publishing ...
 */

export interface Logger {
	info(msg: string): void;
	warn(msg: string): void;
	error(msg: string): void;
}

export function scoped(prefix: string): Logger {
	const tag = `[${prefix}]`;
	return {
		info(msg) {
			console.log(`${tag} ${msg}`);
		},
		warn(msg) {
			console.warn(`${tag} ${msg}`);
		},
		error(msg) {
			console.error(`${tag} ${msg}`);
		},
	};
}
