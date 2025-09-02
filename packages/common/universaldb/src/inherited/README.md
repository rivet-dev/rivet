# Inherited source

Files from this folder were copied from the foundationdbrs crate. Since this repo doesn't directly use FDB, we only copied parts that we needed for the UniversalDB api to work.

This removes the dependency on foundationdb-sys, allowing static compilation with musl.

Origin: https://github.com/foundationdb-rs/foundationdb-rs at 34955a582e964c42c68717b03f97fd0ea3b3cc02