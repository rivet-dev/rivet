# UniversalDB Keys

## Key Structure

Keys are tuples of typed elements packed into bytes. Key constants are defined in `universaldb::prelude::*` (e.g., `ACTOR`, `DATA`, `NAMESPACE`, `WORKFLOW_ID`).

```rust
// Key tuple: (ACTOR, DATA, actor_id, WORKFLOW_ID)
// Packed as bytes for storage
```

## Defining a Key

A key struct needs three trait implementations:

### 1. FormalKey - Value Type & Serialization

```rust
impl FormalKey for MyKey {
    type Value = i64;  // The value type stored with this key

    fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
        Ok(i64::from_be_bytes(raw.try_into()?))
    }

    fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
        Ok(value.to_be_bytes().to_vec())
    }
}
```

### 2. TuplePack - Key Encoding

```rust
impl TuplePack for MyKey {
    fn pack<W: std::io::Write>(
        &self,
        w: &mut W,
        tuple_depth: TupleDepth,
    ) -> std::io::Result<VersionstampOffset> {
        let t = (ACTOR, DATA, self.actor_id, MY_KEY_TYPE);
        t.pack(w, tuple_depth)
    }
}
```

### 3. TupleUnpack - Key Decoding

```rust
impl<'de> TupleUnpack<'de> for MyKey {
    fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
        let (input, (_, _, actor_id, key_type)) =
            <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

        // Validate key type to ensure we're parsing the correct key
        if key_type != MY_KEY_TYPE {
            return Err(PackError::Message("expected MY_KEY_TYPE".into()));
        }

        Ok((input, MyKey { actor_id }))
    }
}
```

## Subspace Keys

Subspace keys define a prefix for range queries. They only implement `TuplePack` (no `FormalKey` or `TupleUnpack`).

```rust
pub struct MySubspaceKey {
    namespace_id: Id,
    name: String,
    create_ts: Option<i64>,  // Optional trailing fields
}

impl MySubspaceKey {
    pub fn new(namespace_id: Id, name: String) -> Self {
        Self { namespace_id, name, create_ts: None }
    }

    pub fn with_create_ts(namespace_id: Id, name: String, create_ts: i64) -> Self {
        Self { namespace_id, name, create_ts: Some(create_ts) }
    }
}

impl TuplePack for MySubspaceKey {
    fn pack<W: std::io::Write>(&self, w: &mut W, tuple_depth: TupleDepth) -> std::io::Result<VersionstampOffset> {
        let mut offset = VersionstampOffset::None { size: 0 };

        let t = (NAMESPACE, self.namespace_id, &self.name);
        offset += t.pack(w, tuple_depth)?;

        // Pack optional trailing fields
        if let Some(create_ts) = &self.create_ts {
            offset += create_ts.pack(w, tuple_depth)?;
        }

        Ok(offset)
    }
}
```

## Linking Keys to Subspaces

Provide helper methods to create subspace keys from the main key:

```rust
impl MyKey {
    pub fn subspace(namespace_id: Id, name: String) -> MySubspaceKey {
        MySubspaceKey::new(namespace_id, name)
    }

    pub fn subspace_with_create_ts(namespace_id: Id, name: String, create_ts: i64) -> MySubspaceKey {
        MySubspaceKey::with_create_ts(namespace_id, name, create_ts)
    }
}
```

## Usage with Transactions

```rust
// Writing
tx.write(&MyKey::new(actor_id), my_value)?;

// Reading
let value = tx.read(&MyKey::new(actor_id), Serializable).await?;

// Range query over subspace
let subspace = keys::subspace().subspace(&MyKey::subspace(ns_id, name));
let (start, end) = subspace.range();

let mut stream = tx.get_ranges_keyvalues(
    (start, end).into(),
    Snapshot,
);

while let Some(entry) = stream.try_next().await? {
    let (key, value) = tx.read_entry::<MyKey>(&entry)?;
}
```

## Key Type Validation

When iterating over a subspace containing multiple key types, validate in `TupleUnpack`:

```rust
// Keys in (ACTOR, DATA, actor_id, KEY_TYPE) share a prefix
// Validate KEY_TYPE to filter during iteration
if key_type != WORKFLOW_ID {
    return Err(PackError::Message("expected WORKFLOW_ID key type".into()));
}
```

This prevents deserializing the wrong value type when keys share a common prefix.
