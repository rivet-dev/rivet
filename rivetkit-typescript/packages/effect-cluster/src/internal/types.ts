/**
 * Wire shapes shared by the cluster client and the entity actor.
 *
 * @internal
 */

/** Delivery options sent with an entity request. */
export interface EntityDeliveryOptions {
	readonly deliverAt?: number | undefined;
	readonly primaryKey?: string | null | undefined;
	readonly replyTo?: string | undefined;
}

/** Result of the entity actor's `invoke` action. */
export type EntityInvokeResult =
	| {
			readonly _tag: "Success";
			readonly requestId: string;
			readonly replies: ReadonlyArray<string>;
	  }
	| {
			readonly _tag: "MailboxFull";
	  }
	| {
			readonly _tag: "EncodedMessageTooLarge";
	  }
	| {
			readonly _tag: "AskDeduplicatedToTell";
	  };
