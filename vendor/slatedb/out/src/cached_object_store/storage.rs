use async_trait::async_trait;
use bytes::Bytes;
use object_store::{path::Path, Attribute, Attributes, ObjectMeta};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, ops::Range};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalCacheHead {
    pub location: String,
    pub last_modified: String,
    pub size: u64,
    pub e_tag: Option<String>,
    pub version: Option<String>,
    pub attributes: HashMap<String, String>,
}

impl LocalCacheHead {
    pub fn meta(&self) -> ObjectMeta {
        ObjectMeta {
            location: self.location.clone().into(),
            last_modified: self.last_modified.parse().unwrap_or_default(),
            size: self.size,
            e_tag: self.e_tag.clone(),
            version: self.version.clone(),
        }
    }

    pub fn attributes(&self) -> Attributes {
        let mut attrs = Attributes::new();
        for (key, value) in self.attributes.iter() {
            let key = match key.as_str() {
                "Cache-Control" => Attribute::CacheControl,
                "Content-Disposition" => Attribute::ContentDisposition,
                "Content-Encoding" => Attribute::ContentEncoding,
                "Content-Language" => Attribute::ContentLanguage,
                "Content-Type" => Attribute::ContentType,
                _ => Attribute::Metadata(key.to_string().into()),
            };
            let value = value.to_string().into();
            attrs.insert(key, value);
        }
        attrs
    }
}

impl From<(&ObjectMeta, &Attributes)> for LocalCacheHead {
    fn from((meta, attrs): (&ObjectMeta, &Attributes)) -> Self {
        let mut attrs_map = HashMap::new();
        for (key, value) in attrs.iter() {
            let key = match key {
                Attribute::CacheControl => "Cache-Control",
                Attribute::ContentDisposition => "Content-Disposition",
                Attribute::ContentEncoding => "Content-Encoding",
                Attribute::ContentLanguage => "Content-Language",
                Attribute::ContentType => "Content-Type",
                Attribute::Metadata(key) => key,
                _ => continue,
            };
            attrs_map.insert(key.to_string(), value.to_string());
        }
        LocalCacheHead {
            location: meta.location.to_string(),
            last_modified: meta.last_modified.to_rfc3339(),
            size: meta.size,
            e_tag: meta.e_tag.clone(),
            version: meta.version.clone(),
            attributes: attrs_map,
        }
    }
}

#[async_trait]
pub trait LocalCacheStorage: Send + Sync + std::fmt::Debug + Display + 'static {
    fn entry(&self, location: &Path, part_size: usize) -> Box<dyn LocalCacheEntry>;

    async fn start_evictor(&self);
}

#[async_trait]
pub trait LocalCacheEntry: Send + Sync + std::fmt::Debug + 'static {
    async fn save_part(&self, part_number: PartID, buf: Bytes) -> object_store::Result<()>;

    async fn read_part(
        &self,
        part_number: PartID,
        range_in_part: Range<usize>,
    ) -> object_store::Result<Option<Bytes>>;

    /// might be useful on rewriting GET request on the prefetch phase. the cached files are
    /// expected to be in the same folder, so it'd be expected to be fast without expensive
    /// globbing.
    #[cfg(test)]
    async fn cached_parts(&self) -> object_store::Result<Vec<PartID>>;

    async fn save_head(&self, meta: (&ObjectMeta, &Attributes)) -> object_store::Result<()>;

    async fn read_head(&self) -> object_store::Result<Option<(ObjectMeta, Attributes)>>;

    /// Deletes this cache entry from the associated cache on the best effort
    /// basis. If some error happens during the deletion, it's logged instead
    /// of being reported to the caller.
    async fn delete(&self);
}

pub type PartID = usize;
