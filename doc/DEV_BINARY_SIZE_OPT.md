# Binary Compression Performance

## Goals

Since we upload our binaries to the cloud on the fly, the size of our binaries
is very important without sacrificing build time.

The two main factors we can tweak are:

1. Build optimizations
2. Compression

Often times, compression will be able to achieve the same size reduction that a
build optimization can but with much less time.

## Compression

### xz

Offers best compression but is _very_ slow. Unless we can find a way to compress faster than tar can, this is too slow

Compresses a 26M binary (`panic=abort,strip`) in 25.0s.

```
tar cvJ target/x86_64-unknown-linux-musl/debug/user-get | wc -c
time tar cvJ target/x86_64-unknown-linux-musl/debug/user-get > /dev/null
```

### bzip

Offers much better compression than gzip, but much faster than xz.

Compresses a 42M binary (`panic=abort`) in 5.2s. Compresses a 26M binary (`panic=abort,strip`) in 2.8s.

```
tar cvj target/x86_64-unknown-linux-musl/debug/user-get | wc -c
time tar cvj target/x86_64-unknown-linux-musl/debug/user-get > /dev/null
```

### Gzip

Measure gzip'd binary size size with: tar cvj target/debug/xxxx | wc -c

Compresses a 42M binary (`panic=abort`) in 4.4s. Compresses a 26M binary (`panic=abort,strip`) in 4.4s.

```
tar cvz target/x86_64-unknown-linux-musl/debug/user-get | wc -c
time tar cvz target/x86_64-unknown-linux-musl/debug/user-get > /dev/null
```

## Optimization

See https://github.com/johnthagen/min-sized-rust

## Results

| Build method            | Incremental build time | Size raw | Size (xz) | Size (bzip) | Size (gzip) |
| ----------------------- | ---------------------- | -------- | --------- | ----------- | ----------- |
| Default                 | 13.7s                  | 48M      | 9.2M      | 12.8M       | 14.8M       |
| panic=abort             | 13.16s                 | 42M      | 7.9M      | 10.6M       | 12.5M       |
| panic=abort strip       | 23.3s                  | 26.2M    | 5.2M      | 7.4M        | 8.2M        |
| panic=abort opt=z       | 10.3s                  | 21M      | 5.1M      | 6.7M        | 7.5M        |
| panic=abort strip opt=z | 10.1s                  | 11.9M    | 3.4M      | 4.6M        | 4.8M        |

## Conclusion

We're opting for `panic=abort strip opt=z` combined with bzip. This is the best
balance of performance and size.

Adding `opt=z` seems to also *improve* incremental build performance which is
unexpected. This hasn't been measured with larger changes.

### Sacrifices

-   `panic=abort`: No backtraces for panics
-   `opt=z`: Likely impacts performance
-   `strip`: No helpful symbols for debugging where crashes happen
    -   Later: investigate [alternative strip settings](https://doc.rust-lang.org/cargo/reference/profiles.html#strip)

