package dev.rivet.kit.internal;

import dev.rivet.kit.Actor;
import dev.rivet.kit.Concurrent;
import dev.rivet.kit.Name;
import dev.rivet.kit.None;
import java.lang.invoke.MethodHandle;
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.lang.reflect.GenericArrayType;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.lang.reflect.TypeVariable;
import java.lang.reflect.WildcardType;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import tools.jackson.databind.JavaType;
import tools.jackson.databind.ObjectReader;
import tools.jackson.databind.ObjectWriter;

/**
 * What an actor API interface declares: its name, actions, events and params,
 * read once per interface and checked eagerly so mistakes fail at registration (or
 * on the client's first use), with a message naming the method.
 */
public final class ApiModel {
    private static final ClassValue<ApiModel> CACHE = new ClassValue<>() {
        @Override
        protected ApiModel computeValue(Class<?> api) {
            return new ApiModel(api);
        }
    };

    public final Class<?> api;
    public final String name;
    final Map<String, Action> actions = new LinkedHashMap<>();
    final Map<Method, Action> byMethod = new LinkedHashMap<>();
    final Type eventType;
    final Map<String, Codec.EventCodec> eventsByName = new LinkedHashMap<>();
    final Map<Class<?>, Codec.EventCodec> eventsByClass = new LinkedHashMap<>();
    /** Null when the actor takes no params. */
    final JavaType paramsType;

    public static ApiModel of(Class<?> api) {
        return CACHE.get(api);
    }

    /** One action: how to decode its args, call it, and encode its result. */
    static final class Action {
        final String name;
        final Method method;
        final Codec.Positional args;
        final JavaType resultType;
        final ObjectWriter resultWriter;
        final ObjectReader resultReader;
        final boolean concurrent;
        final boolean returnsVoid;
        /** {@code (Object impl, Object[] args) -> Object}. */
        final MethodHandle invoker;

        Action(String name, Method method) {
            this.name = name;
            this.method = method;
            Type[] params = method.getGenericParameterTypes();
            String[] names = new String[params.length];
            boolean named = true;
            for (int i = 0; i < params.length; i++) {
                var parameter = method.getParameters()[i];
                named &= parameter.isNamePresent();
                names[i] = parameter.getName();
                checkWireType(params[i], method, "parameter " + (i + 1));
                if (rawOf(params[i]) == Optional.class) {
                    throw bad(method, "Optional parameters are not supported; use a nullable type");
                }
            }
            this.args = new Codec.Positional(params, named ? names : null);
            Type result = method.getGenericReturnType();
            this.returnsVoid = result == void.class || result == Void.class;
            if (!returnsVoid) checkWireType(result, method, "return type");
            this.resultType = Codec.type(returnsVoid ? Object.class : result);
            this.resultWriter = Codec.MAPPER.writerFor(resultType);
            this.resultReader = Codec.MAPPER.readerFor(resultType);
            this.concurrent = method.isAnnotationPresent(Concurrent.class);
            try {
                MethodHandle handle = MethodHandles.publicLookup().unreflect(method);
                this.invoker = handle.asType(MethodType.genericMethodType(params.length + 1))
                        .asSpreader(1, Object[].class, params.length)
                        .asType(MethodType.methodType(Object.class, Object.class, Object[].class));
            } catch (IllegalAccessException e) {
                throw bad(method, "the actor API interface must be public (and exported to dev.rivet.kit when using modules)");
            }
        }

        byte[] encodeResult(Object value) {
            return returnsVoid ? new byte[] {(byte) 0xf6} : resultWriter.writeValueAsBytes(value);
        }

        Object decodeResult(byte[] bytes) {
            if (returnsVoid || bytes.length == 0) return null;
            Object value = resultReader.readValue(bytes);
            if (value == null && method.getReturnType().isPrimitive()) {
                throw new dev.rivet.kit.RivetException("client", "null_result",
                        "action " + name + " returned null for a primitive " + method.getReturnType());
            }
            return value;
        }
    }

    private ApiModel(Class<?> api) {
        if (!api.isInterface()) {
            throw new IllegalArgumentException(api.getName() + " must be an interface extending dev.rivet.kit.Actor");
        }
        if (!Actor.class.isAssignableFrom(api)) {
            throw new IllegalArgumentException(api.getName() + " must extend dev.rivet.kit.Actor<E, P>");
        }
        if (!Modifier.isPublic(api.getModifiers())) {
            throw new IllegalArgumentException(api.getName() + " must be public");
        }
        this.api = api;
        Name named = api.getAnnotation(Name.class);
        this.name = named != null ? named.value() : decapitalize(api.getSimpleName());

        Type[] eventAndParams = resolveActorArgs(api);
        this.eventType = eventAndParams[0];
        Type params = eventAndParams[1];
        this.paramsType = rawOf(params) == None.class ? null : Codec.type(params);
        if (paramsType != null) checkWireType(params, null, "connection params " + params.getTypeName());
        collectEvents(eventType);

        for (Method method : api.getMethods()) {
            if (method.isDefault() || Modifier.isStatic(method.getModifiers()) || method.getDeclaringClass() == Actor.class) {
                continue;
            }
            if (method.getDeclaringClass() == Object.class) continue;
            if (method.getTypeParameters().length > 0) {
                throw bad(method, "generic methods cannot be actions");
            }
            Name actionName = method.getAnnotation(Name.class);
            String wire = actionName != null ? actionName.value() : method.getName();
            if (actions.containsKey(wire)) {
                throw bad(method, "two actions are named `" + wire + "`; overloads are not supported (rename one or use @Name)");
            }
            Action action = new Action(wire, method);
            actions.put(wire, action);
            byMethod.put(method, action);
        }
    }

    Action action(Method method) {
        Action action = byMethod.get(method);
        if (action == null) {
            throw new IllegalArgumentException(method.getName() + " is not an action of " + api.getName());
        }
        return action;
    }

    Codec.EventCodec event(Class<?> type) {
        Codec.EventCodec codec = eventsByClass.get(type);
        if (codec == null) {
            throw new IllegalArgumentException(type.getName() + " is not an event of " + api.getName());
        }
        return codec;
    }

    private void collectEvents(Type type) {
        Class<?> raw = rawOf(type);
        if (raw == None.class) return;
        if (raw.isRecord()) {
            Name named = raw.getAnnotation(Name.class);
            String event = named != null ? named.value() : decapitalize(raw.getSimpleName());
            if (eventsByName.containsKey(event)) {
                throw new IllegalArgumentException("two events of " + api.getName() + " are named `" + event + "`");
            }
            Codec.EventCodec codec = new Codec.EventCodec(raw, event);
            eventsByName.put(event, codec);
            eventsByClass.put(raw, codec);
            return;
        }
        if (raw.isSealed()) {
            for (Class<?> permitted : raw.getPermittedSubclasses()) collectEvents(permitted);
            return;
        }
        throw new IllegalArgumentException("events of " + api.getName() + " must be a record, a sealed interface of"
                + " records, or None; got " + raw.getName());
    }

    /** Finds {@code E} and {@code P} in {@code Actor<E, P>}. */
    private static Type[] resolveActorArgs(Class<?> type) {
        for (Type parent : type.getGenericInterfaces()) {
            if (parent instanceof ParameterizedType p && p.getRawType() == Actor.class) {
                Type[] args = p.getActualTypeArguments();
                for (Type arg : args) {
                    if (arg instanceof TypeVariable<?> || arg instanceof WildcardType) {
                        throw new IllegalArgumentException(type.getName() + " must give Actor concrete type arguments");
                    }
                }
                return args;
            }
            Class<?> raw = rawOf(parent);
            if (raw != null && raw.isInterface() && Actor.class.isAssignableFrom(raw) && raw != Actor.class) {
                return resolveActorArgs(raw);
            }
        }
        throw new IllegalArgumentException(type.getName() + " must extend Actor<E, P> with concrete types");
    }

    static Class<?> rawOf(Type type) {
        if (type instanceof Class<?> c) return c;
        if (type instanceof ParameterizedType p) return (Class<?>) p.getRawType();
        if (type instanceof GenericArrayType g) return rawOf(g.getGenericComponentType()).arrayType();
        return Object.class;
    }

    /** Values must survive JavaScript clients: no non-string map keys, no raw type variables. */
    static void checkWireType(Type type, Method method, String what) {
        if (type instanceof TypeVariable<?>) {
            throw bad(method, what + " uses a type variable; use a concrete type");
        }
        if (type instanceof ParameterizedType p) {
            Class<?> raw = (Class<?>) p.getRawType();
            if (Map.class.isAssignableFrom(raw)) {
                Class<?> key = rawOf(p.getActualTypeArguments()[0]);
                if (key != String.class && !key.isEnum()) {
                    throw bad(method, what + " is a Map with " + key.getSimpleName() + " keys; only String or enum keys"
                            + " survive JSON clients");
                }
            }
            for (Type arg : p.getActualTypeArguments()) checkWireType(arg, method, what);
        }
        if (type instanceof GenericArrayType g) checkWireType(g.getGenericComponentType(), method, what);
    }

    private static IllegalArgumentException bad(Method method, String message) {
        String where = method == null ? "" : method.getDeclaringClass().getName() + "." + method.getName() + ": ";
        return new IllegalArgumentException(where + message);
    }

    static String decapitalize(String name) {
        return name.isEmpty() ? name : Character.toLowerCase(name.charAt(0)) + name.substring(1);
    }
}
