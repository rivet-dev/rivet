package dev.rivet.kit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import java.util.Optional;
import java.util.function.Function;
import org.junit.jupiter.api.Test;

/**
 * Mistakes in an actor API that javac cannot see fail at registration, with a
 * message naming the problem. Uses only the public API; nothing is served.
 */
class RegistryValidationTest {
    public interface WithOptional extends Actor<None, None> {
        void find(Optional<String> name);
    }

    public interface WithTypeVariable<T> extends Actor<None, None> {
        void put(T value);
    }

    public interface GenericActor<E> extends Actor<E, None> {
        void x();
    }

    public interface SameEventNames extends Actor<SameEventNames.Event, None> {
        sealed interface Event {}

        @Name("ping")
        record A() implements Event {}

        @Name("ping")
        record B() implements Event {}
    }

    interface NotPublic extends Actor<None, None> {
        void x();
    }

    public interface Base extends Actor<Base.Tick, Base.Params> {
        record Tick(long n) {}

        record Params(String user) {}
    }

    /** Extends another API interface: inherits its events and params. */
    public interface Derived extends Base {
        long count();
    }

    private static <E, A extends Actor<E, ?>> String error(Class<A> api) {
        Function<ActorContext<A, E>, A> factory = ctx -> null;
        return assertThrows(IllegalArgumentException.class, () -> Registry.builder().actor(api, factory))
                .getMessage();
    }

    @Test
    void optionalParametersAreRejected() {
        assertTrue(error(WithOptional.class).contains("Optional parameters"));
    }

    @Test
    void typeVariablesAreRejected() {
        @SuppressWarnings({"unchecked", "rawtypes"})
        Class<WithTypeVariable<String>> api = (Class) WithTypeVariable.class;
        assertTrue(error(api).contains("type variable"));
        @SuppressWarnings({"unchecked", "rawtypes"})
        Class<GenericActor<Object>> generic = (Class) GenericActor.class;
        assertTrue(error(generic).contains("concrete type arguments"));
    }

    @Test
    void duplicateEventNamesAreRejected() {
        assertTrue(error(SameEventNames.class).contains("named `ping`"));
    }

    @Test
    void apisMustBePublic() {
        assertTrue(error(NotPublic.class).contains("must be public"));
    }

    @Test
    void apisCanExtendOtherApis() {
        Function<ActorContext<Derived, Base.Tick>, Derived> factory = ctx -> null;
        Registry registry = Registry.builder().actor(Derived.class, factory).build();
        assertEquals(List.of("derived"), registry.names());
    }
}
