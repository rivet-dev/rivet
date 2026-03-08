# Engine Notes

## VBARE migrations

When changing a versioned VBARE schema, follow the existing migration pattern.

1. Never edit an existing published `*.bare` schema in place. Add a new versioned schema instead.
2. Update the matching `versioned.rs` like this:
   - If the bytes did not change, deserialize both versions into the new wrapper variant:

   ```rust
   6 | 7 => Ok(ToClientMk2::V7(serde_bare::from_slice(payload)?))
   ```

   - If the bytes did change, write the conversion field by field.

   - Do not do this:

   ```rust
   let bytes = serde_bare::to_vec(&x)?;
   serde_bare::from_slice(&bytes)?
   ```
3. Verify the affected Rust crate still builds.
4. For the runner protocol specifically:
   - Bump both protocol constants together:
     - `engine/sdks/rust/runner-protocol/src/lib.rs` `PROTOCOL_MK2_VERSION`
     - `engine/sdks/typescript/runner/src/mod.ts` `PROTOCOL_VERSION`
   - Update the Rust latest re-export in `engine/sdks/rust/runner-protocol/src/lib.rs` to the new generated module.
