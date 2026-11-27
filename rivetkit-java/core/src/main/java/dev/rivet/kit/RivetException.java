package dev.rivet.kit;

import java.util.concurrent.CompletionException;
import java.util.concurrent.ExecutionException;

/**
 * An error in Rivet's {@code group/code/message} shape, with optional metadata.
 * Clients get actor errors as this; throw {@link UserError} from actor code to send
 * one on purpose.
 */
public class RivetException extends RuntimeException {
    private static final long serialVersionUID = 1L;
    static final String INTERNAL_MESSAGE = "An internal error occurred";

    private final String group;
    private final String code;
    private final String detail;
    private final transient Object metadata;

    public RivetException(String group, String code, String message) {
        this(group, code, message, null, null);
    }

    public RivetException(String group, String code, String message, Object metadata, Throwable cause) {
        super(group + "/" + code + ": " + message, cause);
        this.group = group;
        this.code = code;
        this.detail = message;
        this.metadata = metadata;
    }

    public String group() {
        return group;
    }

    public String code() {
        return code;
    }

    /** The message without the {@code group/code:} prefix. */
    public String detail() {
        return detail;
    }

    /** Structured details from the thrower (decoded CBOR: maps, lists, strings, numbers), or null. */
    public Object metadata() {
        return metadata;
    }

    public boolean isInternal() {
        return "core".equals(group) && "internal_error".equals(code);
    }

    /** Unwraps future wrappers; anything that is not a Rivet error becomes an internal error. */
    public static RivetException from(Throwable t) {
        while ((t instanceof CompletionException || t instanceof ExecutionException) && t.getCause() != null) {
            t = t.getCause();
        }
        if (t instanceof RivetException r) return r;
        return new RivetException("core", "internal_error", INTERNAL_MESSAGE, null, t);
    }
}
