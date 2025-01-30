# AVX

AVX (Advanced Vector Extensions) is a SIMD instruction on Intel CPUs to make
FoundationDB run parallel data processing tasks faster.

## Even & odd versions

Even versions of FoundationDB are compiled without AVX. Odd versions are compiled with it.

For example, [7.1.61](https://github.com/apple/foundationdb/releases/tag/7.1.61) is the AVX version of [7.1.60](https://github.com/apple/foundationdb/releases/tag/7.1.60).

Make sure to pay attention to if you're using a version of FDB with AVX.

## When to use AVX

Use AVX on production Linux servers.

## When not to use AVX

Don't use AVX for Docker images, since QEMU can't emulate AVX and will crash ([source](https://github.com/apple/foundationdb/issues/4111#issuecomment-1284040423)). This will prevent ARM users from being able to run FoundationDB.

This might be easy to fix if we build a Docker image built on the prebuilt AMD & ARM binaries on GitHub.

