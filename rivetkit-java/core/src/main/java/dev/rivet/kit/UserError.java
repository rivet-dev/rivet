package dev.rivet.kit;

/**
 * An error meant for the client, in the {@code user} group. Throw it from an
 * action or hook; the client gets the code, message and metadata as thrown. Any
 * other exception reaches clients as {@code core/internal_error} without details
 * (the details are logged on the server).
 */
public class UserError extends RivetException {
    private static final long serialVersionUID = 1L;

    public UserError(String code, String message) {
        super("user", code, message, null, null);
    }

    public UserError(String code, String message, Object metadata) {
        super("user", code, message, metadata, null);
    }
}
