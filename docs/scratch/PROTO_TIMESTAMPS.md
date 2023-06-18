# Proto Timestamps

Proto timestamps were originally stored as `uint64` but changed to `int64` to allow for storage of negative timestamps.

All new timestamps should use the type `sint64` due to its increased efficiency in comparison to `int64`.

The reason `uint64` types were not converted to `sint64` types is because the two types are not in-place compatible.

https://developers.google.com/protocol-buffers/docs/proto3#updating
