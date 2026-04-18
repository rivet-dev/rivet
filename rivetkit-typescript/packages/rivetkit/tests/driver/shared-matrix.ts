import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterAll, describe } from "vitest";
import {
	getDriverRegistryVariants,
	type DriverRegistryVariant,
} from "../driver-registry-variants";
import {
	createNativeDriverTestConfig,
	releaseSharedEngine,
} from "./shared-harness";
import type { DriverTestConfig } from "./shared-types";

const describeDriverSuite =
	process.env.RIVETKIT_DRIVER_TEST_PARALLEL === "1"
		? describe
		: describe.sequential;
const TEST_DIR = join(dirname(fileURLToPath(import.meta.url)), "..");

export interface DriverMatrixOptions {
	registryVariants?: DriverRegistryVariant["name"][];
	encodings?: Array<NonNullable<DriverTestConfig["encoding"]>>;
	config?: Pick<DriverTestConfig, "features" | "skip">;
}

export function describeDriverMatrix(
	suiteName: string,
	defineTests: (driverTestConfig: DriverTestConfig) => void,
	options: DriverMatrixOptions = {},
) {
	const registryVariantNames = new Set(options.registryVariants);
	const variants = getDriverRegistryVariants(TEST_DIR).filter(
		(variant) =>
			registryVariantNames.size === 0 || registryVariantNames.has(variant.name),
	);
	const encodings = options.encodings ?? ["bare", "cbor", "json"];

	describeDriverSuite(suiteName, () => {
		for (const variant of variants) {
			if (variant.skip) {
				describe.skip(`${variant.name} registry`, () => {});
				continue;
			}

			describeDriverSuite(`${variant.name} registry`, () => {
				afterAll(async () => {
					await releaseSharedEngine();
				});

				for (const encoding of encodings) {
					describeDriverSuite(`encoding (${encoding})`, () => {
						defineTests(
							createNativeDriverTestConfig({
								variant,
								encoding,
								...options.config,
							}),
						);
					});
				}
			});
		}
	});
}
