// TODO: Decode directly in to a `Uuid` type so we don't do any heap allocations
// and extra work when debugging

// MARK: Expanded prost::Message
//
// Generated using cargo-expand with prost 0.9

// `Uuid` must be a tuple so any appended serde derivation will derive directly
// to/from a string.

#[derive(Copy, Clone, PartialEq)]
$extra
struct Uuid(uuid::Uuid);

impl ::prost::Message for Uuid {
    #[allow(unused_variables)]
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: ::prost::bytes::BufMut,
    {
        // TODO: Do this without a heap allocation
        let uuid_bytes = self.0.as_bytes().to_vec();
        ::prost::encoding::bytes::encode(1u32, &uuid_bytes, buf);
    }
    #[allow(unused_variables)]
    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: ::prost::encoding::WireType,
        buf: &mut B,
        ctx: ::prost::encoding::DecodeContext,
    ) -> ::core::result::Result<(), ::prost::DecodeError>
    where
        B: ::prost::bytes::Buf,
    {
        const STRUCT_NAME: &str = "Uuid";
        match tag {
            1u32 => {
                // TODO: Do this without a heap allocation
                let mut value = Vec::<u8>::with_capacity(16);
                let res = ::prost::encoding::bytes::merge(wire_type, &mut value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "uuid");
                        error
                    },
                );
                self.0 = uuid::Uuid::from_slice(&value)
                    .map_err(|_| ::prost::DecodeError::new("unable to parse uuid from slice"))?;
                res
            }
            _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        // TODO: Do this without heap allocation
        ::prost::encoding::bytes::encoded_len(1u32, &self.0.as_bytes().to_vec())
    }
    fn clear(&mut self) {
        self.0 = uuid::Uuid::nil();
    }
}
impl ::core::default::Default for Uuid {
    fn default() -> Self {
        Uuid(uuid::Uuid::nil())
    }
}

// MARK: Override debug_tuple
//
// This overrides the default generated Debug impl from `prost::Message`.
impl ::core::fmt::Debug for Uuid {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        f.debug_tuple("Uuid").field(&self.0).finish()
    }
}

// MARK: Custom impls
impl std::fmt::Display for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Uuid {
    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl ::std::convert::From<uuid::Uuid> for Uuid {
    fn from(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }
}

impl ::std::convert::From<Uuid> for uuid::Uuid {
    fn from(uuid: Uuid) -> Self {
        uuid.0
    }
}

impl ::std::ops::Deref for Uuid {
    type Target = uuid::Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
