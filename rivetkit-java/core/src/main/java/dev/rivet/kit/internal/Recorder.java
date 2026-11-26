package dev.rivet.kit.internal;

import java.lang.reflect.Method;
import java.lang.reflect.Proxy;

/**
 * Turns a method reference like {@code ChatRoom::purge} into the action it names
 * and its arguments, by applying it to a proxy that records the call. No
 * serialization tricks: javac has already checked the reference and argument
 * types, and generics are complete.
 */
final class Recorder {
    private Recorder() {}

    /** Types the recording proxy as the API a method reference expects. */
    @SuppressWarnings("unchecked")
    static <T> T as(Object proxy) {
        return (T) proxy;
    }

    /** The recorded call. */
    record Recorded(ApiModel.Action action, Object[] args) {}

    @FunctionalInterface
    interface Invocation {
        void apply(Object proxy) throws Exception;
    }

    private static final ClassValue<Object> PROXIES = new ClassValue<>() {
        @Override
        protected Object computeValue(Class<?> api) {
            return Proxy.newProxyInstance(api.getClassLoader(), new Class<?>[] {api}, (proxy, method, args) -> {
                if (method.getDeclaringClass() == Object.class) {
                    throw new IllegalArgumentException("method references must name an action, not " + method.getName());
                }
                throw new Captured(method, args == null ? new Object[0] : args);
            });
        }
    };

    private static final class Captured extends RuntimeException {
        private static final long serialVersionUID = 1L;
        final transient Method method;
        final transient Object[] args;

        Captured(Method method, Object[] args) {
            super(null, null, false, false);
            this.method = method;
            this.args = args;
        }
    }

    static Recorded record(ApiModel model, Invocation invocation) {
        Object proxy = PROXIES.get(model.api);
        try {
            invocation.apply(proxy);
        } catch (Captured captured) {
            if (captured.method.isDefault()) {
                throw new IllegalArgumentException(captured.method.getName() + " is a default method, not an action");
            }
            return new Recorded(model.action(captured.method), captured.args);
        } catch (ClassCastException e) {
            throw new IllegalArgumentException(
                    "the method reference does not belong to " + model.api.getName() + "; schedules and crons call"
                            + " this actor's own actions",
                    e);
        } catch (Exception e) {
            throw new IllegalArgumentException("not a method reference to an action of " + model.api.getName(), e);
        }
        throw new IllegalArgumentException("not a method reference to an action of " + model.api.getName());
    }
}
