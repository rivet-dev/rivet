package dev.rivet.kit;

import java.util.List;
import java.util.Optional;
import java.util.function.Function;

/**
 * The actor's SQLite database. Parameters bind positionally ({@code ?}): Long,
 * Integer, Short, Byte, Double, Float, Boolean (as 0/1), String, byte[] (blob),
 * Enum (by name) and null.
 *
 * <p>Rows map to {@code T}: a record or class by column name ({@code created_at}
 * also matches {@code createdAt}), or a single-column scalar (Long, String, ...).
 */
public interface Db {
    <T> List<T> query(Class<T> row, String sql, Object... params);

    <T> Optional<T> queryOne(Class<T> row, String sql, Object... params);

    /** One statement; returns rows changed and the last inserted row id. */
    Exec exec(String sql, Object... params);

    /** Several statements separated by {@code ;}, no parameters (schema setup). */
    void script(String sql);

    /** Runs statements in one transaction. */
    List<Exec> batch(List<Statement> statements);

    /** Runs {@code work} in a transaction: commits when it returns, rolls back when it throws. */
    <T> T transaction(Function<Db, T> work);

    record Exec(long changes, Long lastInsertRowId) {}

    record Statement(String sql, List<Object> params) {
        public static Statement of(String sql, Object... params) {
            return new Statement(sql, List.of(params));
        }
    }
}
