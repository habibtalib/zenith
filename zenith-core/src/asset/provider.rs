//! Asset sourcing layer for Zenith.
//!
//! Provides a deterministic, file-IO-free registry for resolving asset bytes
//! by stable id. All ordering-sensitive collections use `BTreeMap` for
//! determinism. No external crate dependencies — only `std`.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::ast::AssetKind;

/// Resolved asset bytes, ready for rendering or embedding. Cheap to clone (`Arc`).
#[derive(Clone)]
pub struct AssetData {
    /// Stable identifier, e.g. `"asset.logo"`.
    pub id: String,
    /// Raw asset file bytes.
    pub bytes: Arc<[u8]>,
    /// The asset kind (image, svg, font, …).
    pub kind: AssetKind,
}

impl std::fmt::Debug for AssetData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetData")
            .field("id", &self.id)
            .field("bytes_len", &self.bytes.len())
            .field("kind", &self.kind)
            .finish()
    }
}

/// Resolve asset bytes by stable id.
///
/// Implementations must never perform file I/O. The CLI (or other callers)
/// load bytes externally and register them before passing the provider in.
pub trait AssetProvider {
    /// Resolve an asset by its stable id.
    ///
    /// Returns `None` if no asset with the given id has been registered.
    #[must_use]
    fn by_id(&self, id: &str) -> Option<AssetData>;
}

/// In-memory asset registry. Register assets up front; this implementation
/// never scans the filesystem.
#[derive(Default)]
pub struct BytesAssetProvider {
    by_id: BTreeMap<String, AssetData>,
}

impl std::fmt::Debug for BytesAssetProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BytesAssetProvider")
            .field("registered_assets", &self.by_id.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl BytesAssetProvider {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_id: BTreeMap::new(),
        }
    }

    /// Register an asset. If an asset with the same `id` already exists, the
    /// most recent registration wins (bytes and kind are replaced).
    pub fn register(&mut self, id: &str, kind: AssetKind, bytes: Arc<[u8]>) {
        let data = AssetData {
            id: id.to_owned(),
            bytes,
            kind,
        };
        self.by_id.insert(data.id.clone(), data);
    }
}

impl AssetProvider for BytesAssetProvider {
    fn by_id(&self, id: &str) -> Option<AssetData> {
        self.by_id.get(id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_bytes(fill: u8, len: usize) -> Arc<[u8]> {
        Arc::from(vec![fill; len].as_slice())
    }

    #[test]
    fn provider_new_is_empty() {
        let p = BytesAssetProvider::new();
        assert!(p.by_id("asset.missing").is_none());
    }

    #[test]
    fn provider_register_and_by_id() {
        let mut p = BytesAssetProvider::new();
        let bytes_a = dummy_bytes(0xAA, 4);
        let bytes_b = dummy_bytes(0xBB, 8);

        p.register("asset.logo", AssetKind::Svg, bytes_a.clone());
        p.register("asset.hero", AssetKind::Image, bytes_b.clone());

        let logo = p.by_id("asset.logo").expect("asset.logo must be found");
        assert_eq!(logo.id, "asset.logo");
        assert_eq!(logo.kind, AssetKind::Svg);
        assert_eq!(logo.bytes[0], 0xAA);
        assert_eq!(logo.bytes.len(), 4);

        let hero = p.by_id("asset.hero").expect("asset.hero must be found");
        assert_eq!(hero.id, "asset.hero");
        assert_eq!(hero.kind, AssetKind::Image);
        assert_eq!(hero.bytes[0], 0xBB);
        assert_eq!(hero.bytes.len(), 8);
    }

    #[test]
    fn provider_unknown_id_returns_none() {
        let mut p = BytesAssetProvider::new();
        p.register("asset.a", AssetKind::Font, dummy_bytes(0, 1));
        assert!(p.by_id("asset.does-not-exist").is_none());
    }

    #[test]
    fn provider_re_register_overwrites() {
        let mut p = BytesAssetProvider::new();
        p.register("asset.x", AssetKind::Image, dummy_bytes(0x01, 2));
        p.register("asset.x", AssetKind::Svg, dummy_bytes(0x02, 3));

        let data = p.by_id("asset.x").expect("must be found");
        assert_eq!(data.kind, AssetKind::Svg);
        assert_eq!(data.bytes[0], 0x02);
        assert_eq!(data.bytes.len(), 3);
    }

    #[test]
    fn provider_by_id_clones_independently() {
        let mut p = BytesAssetProvider::new();
        p.register("asset.a", AssetKind::Image, dummy_bytes(0xFF, 4));

        let d1 = p.by_id("asset.a").expect("first lookup");
        let d2 = p.by_id("asset.a").expect("second lookup");
        // Arc clones share bytes but are independent references.
        assert_eq!(d1.bytes.len(), d2.bytes.len());
        assert_eq!(d1.id, d2.id);
    }
}
