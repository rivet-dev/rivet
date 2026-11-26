package dev.rivet.kit;

import java.util.Optional;

/**
 * Before a connection opens. Throw (for example a {@link UserError}) to reject it.
 *
 * @param <P> the API's connection params type
 */
public interface OnBeforeConnect<P> {
    void onBeforeConnect(P params, Optional<Request> request) throws Exception;
}
