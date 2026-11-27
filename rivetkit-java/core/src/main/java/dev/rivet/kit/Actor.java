package dev.rivet.kit;

/**
 * Marks an interface as an actor's API: what clients can call and what the actor
 * emits. Clients and the actor class share it.
 *
 * <pre>{@code
 * public interface ChatRoom extends Actor<ChatRoom.Event, ChatRoom.Params> {
 *     Message send(String text);                 // an action: plain method, typed args and result
 *     List<Message> history(int limit);
 *
 *     record Params(String user) {}              // connection params (Accepts)
 *     record Message(String sender, String text, long at) {}
 *     sealed interface Event {}                  // events the actor broadcasts
 *     record NewMessage(Message message) implements Event {}
 * }
 * }</pre>
 *
 * <p>Actions are the interface's abstract methods; their names are the method
 * names (or {@link Name}). Overloads and generic methods are rejected at
 * registration. Use {@link None} for an actor without events or params.
 *
 * <p>The actor's name defaults to the interface's simple name with a lowercase
 * first letter ({@code chatRoom}); {@link Name} on the interface overrides it.
 *
 * @param <E> the event type: a sealed interface of records, a single record, or
 *     {@link None}. An event's name is its record's simple name with a lowercase
 *     first letter, and its arguments are the record's components, in order.
 * @param <P> the connection params type, or {@link None}
 */
public interface Actor<E, P> {}
