//! Krdanta listing (F2): per dhatu, all krts present with
//! `base,m,f,n` exactly as JSON. Missing krts are ignored, never error.

use crate::engine::data::DhatuFile;

/// Category order for display (SPEC §4: `krut` first, then derivatives).
pub const KRT_CATEGORIES: [&str; 5] =
    ["krut", "san_krut", "nich_krut", "yang_krut", "yangluk_krut"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KrtItem {
    pub category: String,
    pub krt: String,
    pub base: String,
    pub m: String,
    pub f: String,
    pub n: String,
}

/// All krt entries present for this dhatu, in category order.
#[must_use]
pub fn krt_list(f: &DhatuFile) -> Vec<KrtItem> {
    let mut out = Vec::new();
    for cat in KRT_CATEGORIES {
        if let Some(krts) = f.participles.get(cat) {
            let mut names: Vec<&String> = krts.keys().collect();
            names.sort();
            for krt in names {
                if let Some(arr) = krts.get(krt) {
                    for e in arr {
                        out.push(KrtItem {
                            category: cat.to_owned(),
                            krt: krt.clone(),
                            base: e.base.clone(),
                            m: e.m.clone(),
                            f: e.f.clone(),
                            n: e.n.clone(),
                        });
                    }
                }
            }
        }
    }
    out
}

/// Non-empty declinable cells (`m/f/n`, split on `/`) of one item.
#[must_use]
pub fn declinables(item: &KrtItem) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    for (linga, cell) in [
        ("m", item.m.as_str()),
        ("f", item.f.as_str()),
        ("n", item.n.as_str()),
    ] {
        for v in cell.split('/').filter(|s| !s.is_empty()) {
            out.push((linga, v.to_owned()));
        }
    }
    out
}
