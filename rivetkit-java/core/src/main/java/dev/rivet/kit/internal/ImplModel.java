package dev.rivet.kit.internal;

import dev.rivet.kit.None;
import dev.rivet.kit.OnBeforeConnect;
import dev.rivet.kit.OnConnect;
import dev.rivet.kit.OnCreate;
import dev.rivet.kit.OnDisconnect;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import tools.jackson.databind.JavaType;

/**
 * What an actor class implements, read once per class: the create-input type and
 * a check that hook type arguments match the API (an {@code OnConnect<P>} whose
 * {@code P} differs from the API's params would never see real params).
 */
final class ImplModel {
    private static final ClassValue<ImplModel> CACHE = new ClassValue<>() {
        @Override
        protected ImplModel computeValue(Class<?> type) {
            return new ImplModel(type);
        }
    };

    private final Class<?> type;
    private final JavaType inputType;

    static ImplModel of(Class<?> type) {
        return CACHE.get(type);
    }

    private ImplModel(Class<?> type) {
        this.type = type;
        Type input = typeArgument(type, OnCreate.class);
        this.inputType = input == null || ApiModel.rawOf(input) == None.class ? null : Codec.type(input);
    }

    Object decodeInput(byte[] body) {
        return inputType == null || body.length == 0 ? null : Codec.decode(body, inputType);
    }

    void check(ApiModel api) {
        for (Class<?> hook : new Class<?>[] {OnBeforeConnect.class, OnConnect.class, OnDisconnect.class}) {
            Type params = typeArgument(type, hook);
            if (params == null) continue;
            Class<?> declared = ApiModel.rawOf(params);
            Class<?> expected = api.paramsType == null ? None.class : api.paramsType.getRawClass();
            if (declared != Object.class && declared != expected) {
                throw new IllegalStateException(type.getName() + " implements " + hook.getSimpleName() + "<"
                        + declared.getSimpleName() + "> but " + api.api.getSimpleName() + " accepts "
                        + expected.getSimpleName());
            }
        }
    }

    /** The type argument {@code type} gives {@code generic}, if it implements it directly or via superclasses. */
    static Type typeArgument(Class<?> type, Class<?> generic) {
        for (Class<?> c = type; c != null && c != Object.class; c = c.getSuperclass()) {
            for (Type parent : c.getGenericInterfaces()) {
                if (parent instanceof ParameterizedType p && p.getRawType() == generic) {
                    return p.getActualTypeArguments()[0];
                }
            }
        }
        return null;
    }
}
