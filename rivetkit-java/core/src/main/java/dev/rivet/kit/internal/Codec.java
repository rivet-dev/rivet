package dev.rivet.kit.internal;

import dev.rivet.kit.RivetException;
import java.io.ByteArrayOutputStream;
import java.lang.invoke.MethodHandle;
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.lang.reflect.RecordComponent;
import java.lang.reflect.Type;
import java.util.Arrays;
import java.util.Iterator;
import tools.jackson.core.JsonGenerator;
import tools.jackson.core.JsonParser;
import tools.jackson.core.JsonToken;
import tools.jackson.databind.DeserializationFeature;
import tools.jackson.databind.JavaType;
import tools.jackson.databind.JsonNode;
import tools.jackson.databind.ObjectReader;
import tools.jackson.databind.ObjectWriter;
import tools.jackson.databind.cfg.DateTimeFeature;
import tools.jackson.dataformat.cbor.CBORMapper;

/**
 * CBOR for wire headers and user values. The configuration is pinned: no module
 * discovery (the classpath must not change the wire format), ISO-8601 strings for
 * dates (what JavaScript clients read), no silent null-to-zero for primitives.
 */
public final class Codec {
    static final CBORMapper MAPPER = CBORMapper.builder()
            .enable(DeserializationFeature.FAIL_ON_NULL_FOR_PRIMITIVES)
            .disable(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES)
            .disable(DeserializationFeature.ACCEPT_FLOAT_AS_INT)
            // Arguments are read one element at a time from a shared parser.
            .disable(DeserializationFeature.FAIL_ON_TRAILING_TOKENS)
            .disable(DateTimeFeature.WRITE_DATES_AS_TIMESTAMPS)
            .disable(DateTimeFeature.WRITE_DURATIONS_AS_TIMESTAMPS)
            .build();

    private static final byte[] EMPTY_ARRAY = {(byte) 0x80};

    private Codec() {}

    public static byte[] encode(Object value) {
        return MAPPER.writeValueAsBytes(value);
    }

    public static <T> T decode(byte[] bytes, Class<T> type) {
        if (bytes.length == 0) bytes = new byte[] {(byte) 0xa0};
        return MAPPER.readValue(bytes, type);
    }

    public static <T> T decode(byte[] bytes, JavaType type) {
        return MAPPER.readValue(bytes, type);
    }

    public static JavaType type(Type type) {
        return MAPPER.constructType(type);
    }

    public static <T> T convert(Object value, JavaType type) {
        return MAPPER.convertValue(value, type);
    }

    static RivetException invalidArgs(String what, Throwable cause) {
        return new RivetException("actor", "invalid_args", what, null, cause);
    }

    /**
     * Positional arguments: a CBOR array, one element per parameter, each read and
     * written with its declared generic type. Readers and writers are built once.
     * Also accepts a single map keyed by parameter name (the Rust client's struct
     * form), when that is unambiguous.
     */
    public static final class Positional {
        private final String[] names;
        private final JavaType[] types;
        private final ObjectReader[] readers;
        private final ObjectWriter[] writers;
        private final boolean byNameAllowed;

        public Positional(Type[] types, String[] names) {
            this.names = names;
            this.types = new JavaType[types.length];
            this.readers = new ObjectReader[types.length];
            this.writers = new ObjectWriter[types.length];
            for (int i = 0; i < types.length; i++) {
                this.types[i] = MAPPER.constructType(types[i]);
                this.readers[i] = MAPPER.readerFor(this.types[i]);
                this.writers[i] = MAPPER.writerFor(this.types[i]);
            }
            // A single struct argument is ambiguous with the by-name form.
            this.byNameAllowed = names != null
                    && (types.length > 1 || (types.length == 1 && isScalar(this.types[0])));
        }

        public byte[] encode(Object[] args) {
            if (args == null || args.length == 0) return EMPTY_ARRAY.clone();
            ByteArrayOutputStream out = new ByteArrayOutputStream(64);
            try (JsonGenerator g = MAPPER.createGenerator(out)) {
                g.writeStartArray(null, args.length);
                for (int i = 0; i < args.length; i++) writers[i].writeValue(g, args[i]);
                g.writeEndArray();
            }
            return out.toByteArray();
        }

        public Object[] decode(byte[] bytes) {
            Object[] args = new Object[types.length];
            if (bytes == null || bytes.length == 0) {
                if (types.length == 0) return args;
                throw invalidArgs("expected " + types.length + " arguments, got none", null);
            }
            try (JsonParser p = MAPPER.createParser(bytes)) {
                if (p.nextToken() != JsonToken.START_ARRAY) {
                    throw invalidArgs("arguments must be an array", null);
                }
                JsonToken t = p.nextToken();
                int i = 0;
                if (byNameAllowed && t == JsonToken.START_OBJECT) {
                    JsonNode object = MAPPER.readTree(p);
                    t = p.nextToken();
                    // One map and nothing else: the Rust client's struct form.
                    if (t == JsonToken.END_ARRAY) return decodeByName(object);
                    // Otherwise the map is simply the first positional argument.
                    args[0] = readers[0].readValue(object);
                    i = 1;
                }
                for (; t != JsonToken.END_ARRAY && t != null; t = p.nextToken(), i++) {
                    if (i < types.length) {
                        args[i] = readers[i].readValue(p);
                    } else {
                        p.skipChildren(); // extra arguments are ignored, as in JavaScript
                    }
                }
                for (; i < types.length; i++) {
                    if (types[i].isPrimitive()) {
                        throw invalidArgs("missing argument " + (i + 1) + " (" + name(i) + ")", null);
                    }
                }
                return args;
            } catch (RivetException e) {
                throw e;
            } catch (RuntimeException e) {
                throw invalidArgs("could not decode arguments: " + e.getMessage(), e);
            }
        }

        private Object[] decodeByName(JsonNode object) {
            Object[] args = new Object[types.length];
            for (int i = 0; i < types.length; i++) {
                JsonNode value = object.get(names[i]);
                if (value == null || value.isNull()) {
                    if (types[i].isPrimitive()) throw invalidArgs("missing argument " + names[i], null);
                    continue;
                }
                args[i] = readers[i].readValue(value);
            }
            return args;
        }

        private String name(int i) {
            return names == null ? "arg" + i : names[i];
        }

        private static boolean isScalar(JavaType type) {
            Class<?> raw = type.getRawClass();
            return type.isPrimitive()
                    || Number.class.isAssignableFrom(raw)
                    || raw == String.class
                    || raw == Boolean.class
                    || raw == Character.class
                    || raw.isEnum();
        }
    }

    /**
     * An event record on the wire: its name and its components as positional args.
     * Also accepts a single map keyed by component name (the Rust SDK's form).
     */
    public static final class EventCodec {
        public final Class<?> type;
        public final String name;
        private final Positional args;
        private final MethodHandle[] accessors;
        private final MethodHandle constructor;

        public EventCodec(Class<?> type, String name) {
            this.type = type;
            this.name = name;
            RecordComponent[] components = type.getRecordComponents();
            Type[] types = new Type[components.length];
            String[] names = new String[components.length];
            Class<?>[] raw = new Class<?>[components.length];
            this.accessors = new MethodHandle[components.length];
            MethodHandles.Lookup lookup = MethodHandles.publicLookup();
            try {
                for (int i = 0; i < components.length; i++) {
                    types[i] = components[i].getGenericType();
                    names[i] = components[i].getName();
                    raw[i] = components[i].getType();
                    accessors[i] = lookup.unreflect(components[i].getAccessor());
                }
                this.constructor = lookup.findConstructor(type, MethodType.methodType(void.class, raw));
            } catch (ReflectiveOperationException e) {
                throw new IllegalArgumentException(
                        "event record " + type.getName() + " must be public with a public canonical constructor"
                                + " (and its package exported to dev.rivet.kit when using modules)",
                        e);
            }
            this.args = new Positional(types, names);
        }

        public byte[] encode(Object event) {
            Object[] values = new Object[accessors.length];
            try {
                for (int i = 0; i < accessors.length; i++) values[i] = accessors[i].invoke(event);
            } catch (Throwable t) {
                throw new IllegalStateException("read event " + name, t);
            }
            return args.encode(values);
        }

        public Object decode(byte[] bytes) {
            Object[] values = decodeValues(bytes);
            try {
                return constructor.invokeWithArguments(values);
            } catch (Throwable t) {
                throw new IllegalStateException("build event " + name, t);
            }
        }

        private Object[] decodeValues(byte[] bytes) {
            // The Rust SDK emits a struct as one map argument.
            if (accessors.length != 1 || !args.byNameAllowed) {
                try (JsonParser p = MAPPER.createParser(bytes)) {
                    if (p.nextToken() == JsonToken.START_ARRAY && p.nextToken() == JsonToken.START_OBJECT) {
                        JsonNode object = MAPPER.readTree(p);
                        if (p.nextToken() == JsonToken.END_ARRAY && matchesComponents(object)) {
                            return args.decodeByName(object);
                        }
                    }
                } catch (RuntimeException ignored) {
                    // fall through to the positional form
                }
            }
            return args.decode(bytes);
        }

        private boolean matchesComponents(JsonNode object) {
            if (accessors.length == 1) return false;
            Iterator<String> fields = object.propertyNames().iterator();
            int count = 0;
            while (fields.hasNext()) {
                String field = fields.next();
                if (!Arrays.asList(args.names).contains(field)) return false;
                count++;
            }
            return count > 0;
        }
    }
}
