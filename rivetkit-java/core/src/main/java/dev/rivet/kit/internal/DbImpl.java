package dev.rivet.kit.internal;

import dev.rivet.kit.Db;
import dev.rivet.kit.RivetException;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.function.Function;
import tools.jackson.core.type.TypeReference;
import tools.jackson.databind.JavaType;

/**
 * {@link Db} over the wire. Calls block the calling thread and keep the actor's
 * turn (SQLite is per actor and serialized anyway), so a read-then-write inside one
 * action is atomic.
 */
final class DbImpl implements Db {
    private static final TypeReference<List<List<Object>>> ROWS = new TypeReference<>() {};
    private static final ClassValue<RowMapper> MAPPERS = new ClassValue<>() {
        @Override
        protected RowMapper computeValue(Class<?> type) {
            return new RowMapper(type);
        }
    };

    private final long target;

    DbImpl(long target) {
        this.target = target;
    }

    @Override
    public <T> List<T> query(Class<T> row, String sql, Object... params) {
        Native.Answer answer = Native.callSync(target, Wire.OP_SQL_EXECUTE, new Wire.SqlCall(sql, bind(params)), null);
        Wire.SqlResult result = Codec.decode(answer.header(), Wire.SqlResult.class);
        List<List<Object>> rows = answer.body().length == 0
                ? List.of()
                : Codec.MAPPER.readValue(answer.body(), ROWS);
        RowMapper mapper = MAPPERS.get(row);
        List<T> out = new ArrayList<>(rows.size());
        for (List<Object> values : rows) out.add(row.cast(mapper.map(result.columns(), values)));
        return out;
    }

    @Override
    public <T> Optional<T> queryOne(Class<T> row, String sql, Object... params) {
        List<T> rows = query(row, sql, params);
        return rows.isEmpty() ? Optional.empty() : Optional.ofNullable(rows.get(0));
    }

    @Override
    public Exec exec(String sql, Object... params) {
        Native.Answer answer = Native.callSync(target, Wire.OP_SQL_EXECUTE, new Wire.SqlCall(sql, bind(params)), null);
        Wire.SqlResult result = Codec.decode(answer.header(), Wire.SqlResult.class);
        return new Exec(result.changes(), result.lastInsertRowId());
    }

    @Override
    public void script(String sql) {
        Native.callSync(target, Wire.OP_SQL_EXEC, new Wire.SqlCall(sql, null), null);
    }

    @Override
    public List<Exec> batch(List<Statement> statements) {
        List<Wire.SqlCall> calls = new ArrayList<>(statements.size());
        for (Statement statement : statements) {
            calls.add(new Wire.SqlCall(statement.sql(), bind(statement.params().toArray())));
        }
        Native.Answer answer = Native.callSync(target, Wire.OP_SQL_BATCH, new Wire.SqlBatchCall(calls), null);
        Wire.SqlBatchResult result = Codec.decode(answer.header(), Wire.SqlBatchResult.class);
        List<Exec> out = new ArrayList<>(result.results().size());
        for (Wire.SqlResult r : result.results()) out.add(new Exec(r.changes(), r.lastInsertRowId()));
        return out;
    }

    @Override
    public <T> T transaction(Function<Db, T> work) {
        long tx = Native.handleOf(Native.callSync(target, Wire.OP_SQL_BEGIN, new Wire.SqlBeginCall(null, null), null));
        T result;
        try {
            result = work.apply(new DbImpl(tx));
        } catch (Throwable t) {
            try {
                Native.callSync(tx, Wire.OP_SQL_ROLLBACK, null, null);
            } catch (RuntimeException rollback) {
                t.addSuppressed(rollback);
            }
            throw t;
        }
        Native.callSync(tx, Wire.OP_SQL_COMMIT, null, null);
        return result;
    }

    static List<Object> bind(Object[] params) {
        if (params == null || params.length == 0) return null;
        List<Object> out = new ArrayList<>(params.length);
        for (Object param : params) {
            out.add(switch (param) {
                case null -> null;
                case Long l -> l;
                case Integer i -> i.longValue();
                case Short s -> s.longValue();
                case Byte b -> b.longValue();
                case Double d -> d;
                case Float f -> f.doubleValue();
                case Boolean b -> b ? 1L : 0L;
                case String s -> s;
                case byte[] bytes -> bytes;
                case Enum<?> e -> e.name();
                default -> throw new IllegalArgumentException("unsupported SQL parameter type "
                        + param.getClass().getName() + "; use Long, Integer, Double, Boolean, String, byte[], an enum or null");
            });
        }
        return out;
    }

    /** Maps a row to a type: scalars from the single column, objects by column name. */
    private static final class RowMapper {
        private final JavaType type;
        private final boolean scalar;

        RowMapper(Class<?> type) {
            this.type = Codec.type(type);
            this.scalar = type.isPrimitive() || Number.class.isAssignableFrom(type) || type == String.class
                    || type == Boolean.class || type == byte[].class || type.isEnum() || type == Object.class;
        }

        Object map(List<String> columns, List<Object> values) {
            if (scalar) {
                if (values.size() != 1) {
                    throw new RivetException("actor", "sql_row_shape", "query returned " + values.size()
                            + " columns; a scalar row type needs exactly one");
                }
                return Codec.convert(values.get(0), type);
            }
            Map<String, Object> object = new LinkedHashMap<>();
            for (int i = 0; i < columns.size(); i++) {
                object.put(columns.get(i), values.get(i));
                String camel = camel(columns.get(i));
                if (!camel.equals(columns.get(i))) object.putIfAbsent(camel, values.get(i));
            }
            return Codec.convert(object, type);
        }

        private static String camel(String column) {
            StringBuilder out = new StringBuilder(column.length());
            boolean upper = false;
            for (char c : column.toCharArray()) {
                if (c == '_') {
                    upper = true;
                } else {
                    out.append(upper ? Character.toUpperCase(c) : c);
                    upper = false;
                }
            }
            return out.toString();
        }
    }
}
