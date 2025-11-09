import { describe, expectTypeOf, it } from "vitest";
import type { ActorContext } from "@/actor/contexts/actor";
import type { ActorContextOf, ActorDefinition } from "@/actor/definition";

describe("ActorDefinition", () => {
	describe("ActorContextOf type utility", () => {
		it("should correctly extract the context type from an ActorDefinition", () => {
			// Define some simple types for testing
			interface TestState {
				counter: number;
			}

			interface TestConnParams {
				clientId: string;
			}

			interface TestConnState {
				lastSeen: number;
			}

			interface TestVars {
				foo: string;
			}

			interface TestInput {
				bar: string;
			}

			interface TestDatabase {
				createClient: (ctx: {
					getDatabase: () => Promise<string | unknown>;
				}) => Promise<{ execute: (query: string) => any }>;
				onMigrate: () => void;
			}

			// For testing type utilities, we don't need a real actor instance
			// We just need a properly typed ActorDefinition to check against
			type TestActions = Record<never, never>;
			const dummyDefinition = {} as ActorDefinition<
				TestState,
				TestConnParams,
				TestConnState,
				TestVars,
				TestInput,
				TestDatabase,
				TestActions
			>;

			// Use expectTypeOf to verify our type utility works correctly
			expectTypeOf<
				ActorContextOf<typeof dummyDefinition>
			>().toEqualTypeOf<
				ActorContext<
					TestState,
					TestConnParams,
					TestConnState,
					TestVars,
					TestInput,
					TestDatabase
				>
			>();

			// Make sure that different types are not compatible
			interface DifferentState {
				value: string;
			}

			expectTypeOf<
				ActorContextOf<typeof dummyDefinition>
			>().not.toEqualTypeOf<
				ActorContext<
					DifferentState,
					TestConnParams,
					TestConnState,
					TestVars,
					TestInput,
					TestDatabase
				>
			>();
		});
	});
});
