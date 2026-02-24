# Errors

## RivetError

TODO: Document the derive macro
TODO: Document .build() -> anyhow

## Error chains

- TODO: use `.chain().filter_map(|x| x.downcast_ref::<T>())` otherwise it'll only check the top-most error
- TODO: use `#[source]` in thiserror variants in order for wrapped errors to show up in `.chain()`
- TODO: `#[from]` automatically adds `#[source]`

