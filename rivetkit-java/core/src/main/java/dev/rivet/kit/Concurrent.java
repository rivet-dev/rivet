package dev.rivet.kit;

import java.lang.annotation.Documented;
import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Runs an action without the actor's turn, concurrently with everything else.
 * Use it for reads that are safe in parallel (immutable or concurrent fields, SQL).
 * The method may not touch other mutable fields; {@link ActorContext} calls that
 * need the turn throw.
 */
@Documented
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface Concurrent {}
