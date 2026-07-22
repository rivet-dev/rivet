import { describe, expect, expectTypeOf, test, vi } from "vitest";
import { z } from "zod/v4";
import {
	flattenActionHandlers,
	flattenActionInputSchemas,
} from "../src/actor/actions";
import { actor } from "../src/actor/definition";
import type { ActorDefinitionActions } from "../src/client/actor-common";
import type { ActorHandleRaw } from "../src/client/actor-handle";
import { createActorProxy } from "../src/client/client";

describe("nested actions", () => {
	test("preserves nested handler context and client action types", () => {
		const definition = actor({
			state: { total: 0 },
			actions: {
				calculator: {
					add: (c, amount: number) => {
						c.state.total += amount;
						return c.state.total;
					},
				},
			},
		});
		type ClientActions = ActorDefinitionActions<typeof definition>;

		expectTypeOf<ClientActions["calculator"]["add"]>()
			.parameter(0)
			.toEqualTypeOf<number>();
		expectTypeOf<
			ClientActions["calculator"]["add"]
		>().returns.toEqualTypeOf<Promise<number>>();
	});

	test("dispatches nested proxy calls with dotted names", async () => {
		const action = vi.fn().mockResolvedValue("created");
		const handle = createActorProxy({
			action,
		} as unknown as ActorHandleRaw) as any;

		await expect(handle.users.create({ name: "Ada" })).resolves.toBe(
			"created",
		);
		expect(action).toHaveBeenCalledWith({
			name: "users.create",
			args: [{ name: "Ada" }],
		});
		expect(handle.users.then).toBeUndefined();
		expect(Object.getOwnPropertyDescriptor(handle, "then")).toBeUndefined();
	});

	test("preserves dotted namespace segments when matching nested schemas", () => {
		const create = () => "created";
		const schema = z.tuple([z.string()]);
		const actions = { "admin.users": { create } };

		expect(flattenActionHandlers(actions)).toEqual({
			"admin.users.create": create,
		});
		expect(
			flattenActionInputSchemas(actions, {
				"admin.users": { create: schema },
			}),
		).toEqual({ "admin.users.create": schema });
	});

	test("supports action names that overlap object prototype keys", () => {
		const prototypeAction = () => "prototype";
		const constructorAction = () => "constructor";
		const handlers = flattenActionHandlers({
			["__proto__"]: prototypeAction,
			constructor: constructorAction,
		});

		expect(Object.getPrototypeOf(handlers)).toBeNull();
		expect(handlers.__proto__).toBe(prototypeAction);
		expect(handlers.constructor).toBe(constructorAction);

		const definition = actor({
			actions: { ["__proto__"]: prototypeAction },
		});
		expect(Object.hasOwn(definition.config.actions, "__proto__")).toBe(
			true,
		);
	});

	test("flattens handlers and schemas to dotted names", () => {
		const create = () => "created";
		const schema = z.tuple([z.object({ name: z.string() })]);
		const actions = { users: { create } };

		expect(flattenActionHandlers(actions)).toEqual({
			"users.create": create,
		});
		expect(
			flattenActionInputSchemas(actions, {
				users: { create: schema },
			}),
		).toEqual({ "users.create": schema });
	});

	test("keeps dotted flat action and schema keys compatible", () => {
		const create = () => "created";
		const schema = z.tuple([z.string()]);
		const actions = { "users.create": create };

		expect(flattenActionHandlers(actions)).toEqual({
			"users.create": create,
		});
		expect(
			flattenActionInputSchemas(actions, { "users.create": schema }),
		).toEqual({ "users.create": schema });
	});

	test("rejects colliding flattened names", () => {
		expect(() =>
			flattenActionHandlers({
				"users.create": () => "flat",
				users: { create: () => "nested" },
			}),
		).toThrow("Multiple action definitions flatten to `users.create`");
	});

	test("rejects non-function leaves", () => {
		expect(() => flattenActionHandlers({ users: { create: 42 } })).toThrow(
			"Action `users.create` must be an action handler or group",
		);
	});
});
