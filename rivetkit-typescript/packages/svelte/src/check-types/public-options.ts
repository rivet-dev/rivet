import type {
  ActionDefaults,
  ActorErrorLike,
  AnyActorRegistry,
  RivetContext,
  RivetKit,
  SvelteRivetKitOptions,
} from "../lib/index.js";
import { createRivetKitWithClient, withActorParams } from "../lib/index.js";
import type { Client } from "rivetkit/client";

type IsAny<T> = 0 extends 1 & T ? true : false;
type ExpectFalse<T extends false> = T;

const clientOptions = {
  actionDefaults: { timeout: 5_000, timeoutByAction: { getSnapshot: 1_000 } },
  connectionInspector: true,
} satisfies SvelteRivetKitOptions<AnyActorRegistry>;

export function configureContext(
  context: RivetContext<AnyActorRegistry>,
): RivetKit<AnyActorRegistry> {
  return context.setup(undefined, clientOptions);
}

export function configureActor(rivet: RivetKit<AnyActorRegistry>): void {
  rivet.createReactiveActor({
    name: "chat" as never,
    key: ["room-1"],
    actionDefaults: { guardConnection: true },
  });
}

const client = {} as Client<AnyActorRegistry>;
const directRivet = createRivetKitWithClient<AnyActorRegistry>(client);
type DirectFactoryResultIsNotAny = ExpectFalse<IsAny<typeof directRivet>>;

// A direct factory call must expose the declared RivetKit surface, not `any`.
directRivet.connectionInspector;
// @ts-expect-error Unknown properties must not pass through an `any` result.
directRivet.notPartOfRivetKit;

const getActorOptions = withActorParams<AnyActorRegistry, never>(
  {
    name: "chat" as never,
    key: ["room-1"],
    actionDefaults: { guardConnection: true },
  },
  { token: "secret" },
);
const preservedActionDefaults: ActionDefaults | undefined =
  getActorOptions().actionDefaults;

// Serialized cross-realm actor errors are structural and need not be Error instances.
const serializedActorError: ActorErrorLike = {
  __type: "RivetError",
  group: "user",
  code: "FORBIDDEN",
  message: "Not allowed",
};

void (0 as unknown as DirectFactoryResultIsNotAny);
void preservedActionDefaults;
void serializedActorError;
