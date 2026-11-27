package dev.rivet.kit;

import java.lang.annotation.Documented;
import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * The wire name of an actor (on the API interface), an action (on a method), or an
 * event (on a record). Pin names you must keep stable across renames: clients in
 * other languages and persisted schedules refer to them.
 */
@Documented
@Retention(RetentionPolicy.RUNTIME)
@Target({ElementType.TYPE, ElementType.METHOD})
public @interface Name {
    String value();
}
