# Default Builds

These are builds that get automatically uploaded to Rivet's build registry. These are used for things like the
default build provided when creating a new game & builds used in tests.

These builds are stored as Git LFS blobs.

## Inner workings

These builds are included with the `build-default-create` oneoff service. When this service gets deployed, it
automatically uploads it & registers the default builds in the registry.

Note that this is not built to support _large_ builds by the nature of how big the `build-default-create`
binary would balloon to be.

## Building

To rebuild all builds, run:

```
./scripts/default_builds/build.sh
```

To rebuild a specific build, run:

```
./scripts/default_builds/build.sh my-build
```
