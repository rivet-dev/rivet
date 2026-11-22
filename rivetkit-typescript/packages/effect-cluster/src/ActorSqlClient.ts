/**
 * An `SqlClient` on the current entity actor's own SQLite database.
 *
 * Each entity actor provides this service to its handler build and handlers.
 * Read it with {@link ActorSqlClient.orDie} in an `Entity.toLayer` build so
 * the layer has no extra requirement. It is a separate tag from
 * `SqlClient.SqlClient`, so an application-wide SQL client stays reachable
 * inside handlers.
 *
 * - Rows live and die with the entity's actor and cannot be queried from
 *   outside it or from another entity.
 * - Tables whose names start with `cluster_` are reserved for the mailbox.
 * - `withTransaction` uses one actor SQLite transaction. While it is open,
 *   every other statement on the actor, including mailbox writes for other
 *   requests, waits. Keep transactions to SQL work.
 *
 * @since 3.1.2
 */
import * as Context from "effect/Context";
import * as Effect from "effect/Effect";
import * as Option from "effect/Option";
import type * as SqlClient from "effect/sql/SqlClient";

export class ActorSqlClient extends Context.Service<
	ActorSqlClient,
	SqlClient.SqlClient
>()("@rivetkit/effect-cluster/ActorSqlClient") {
	/**
	 * Reads the client without adding a requirement. Dies when no entity actor
	 * provides it, for example in a test that did not provide one.
	 */
	static readonly orDie: Effect.Effect<SqlClient.SqlClient> = Effect.flatMap(
		Effect.serviceOption(ActorSqlClient),
		Option.match({
			onNone: () =>
				Effect.die(
					new Error(
						"ActorSqlClient is only available inside an entity actor; provide it from a SQL layer in tests",
					),
				),
			onSome: Effect.succeed,
		}),
	);
}
