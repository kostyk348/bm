//! bm lib: закладки с тегами поверх eml-tag.
//!
//! Запись: X-Tag: <тег>*, X-Timestamp: ts, тело {"url","title","note"}.

use emlbox::format::find_blank;
use emlbox::tagdb;
use serde_json::Value;
use std::path::Path;

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn body_of(p: &Path) -> Result<Value, String> {
    let data = std::fs::read(p).map_err(|e| e.to_string())?;
    let (end, sep) = find_blank(&data).ok_or("no blank line in record")?;
    serde_json::from_slice(&data[end + sep..]).map_err(|e| e.to_string())
}

pub fn add(
    db: &Path,
    url: &str,
    title: &str,
    note: &str,
    tags: &[String],
    ts: u64,
) -> Result<String, String> {
    let id = format!("bm_{}", emlbox::format::hash_bytes(url.as_bytes()));
    // merge: re-adding the same url keeps old tags
    let mut merged: Vec<String> = tags.to_vec();
    let q = format!("X-Record-ID == \"{id}\"");
    if let Ok((recs, _)) = tagdb::query(db, &q) {
        if let Some((_, headers)) = recs.first() {
            for (k, v) in headers {
                if k.eq_ignore_ascii_case("X-Tag") && !merged.contains(v) {
                    merged.push(v.clone());
                }
            }
        }
    }
    let body = serde_json::json!({"url": url, "title": title, "note": note});
    tagdb::insert(db, &id, &merged, "local", ts, &body).map_err(|e| e.to_string())?;
    Ok(id)
}

/// (url, title, tags) per match.
pub fn find(db: &Path, q: &str) -> Result<Vec<(String, String, Vec<String>)>, String> {
    let (records, _) = tagdb::query(db, q)?;
    let mut out = Vec::new();
    for (p, headers) in records {
        let tags: Vec<String> = headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("X-Tag"))
            .map(|(_, v)| v.clone())
            .collect();
        let b = body_of(&p).unwrap_or_else(|_| serde_json::json!({}));
        let url = b.get("url").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let title = b.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
        out.push((url, title, tags));
    }
    Ok(out)
}

pub fn tag_counts(db: &Path) -> Result<Vec<(String, usize)>, String> {
    let (records, _) = tagdb::scan(db)?;
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (_, h) in records {
        for (k, v) in h {
            if k.eq_ignore_ascii_case("X-Tag") {
                *counts.entry(v.clone()).or_default() += 1;
            }
        }
    }
    let mut out: Vec<_> = counts.into_iter().collect();
    out.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(tag: &str) -> PathBuf {
        let db = std::env::temp_dir().join(format!("bm_{}_{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&db);
        std::fs::create_dir_all(&db).unwrap();
        db
    }

    #[test]
    fn add_find_tags() {
        let db = setup("t");
        add(&db, "https://example.com/vita", "Vita page", "", &["vita".into(), "docs".into()], 100).unwrap();
        add(&db, "https://example.com/rust", "Rust page", "на заметку", &["rust".into()], 200).unwrap();
        let r = find(&db, "X-Tag == \"vita\"").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, "https://example.com/vita");
        assert_eq!(r[0].1, "Vita page");
        let r = find(&db, "X-Tag == \"rust\" AND X-Tag == \"vita\"").unwrap();
        assert!(r.is_empty());
        assert_eq!(tag_counts(&db).unwrap().len(), 3);
    }

    #[test]
    fn duplicate_url_merges_tags() {
        let db = setup("dup");
        add(&db, "https://x.dev/a", "", "", &["a".into()], 1).unwrap();
        add(&db, "https://x.dev/a", "", "", &["b".into()], 2).unwrap();
        let r = find(&db, "X-Tag == \"a\" AND X-Tag == \"b\"").unwrap();
        assert_eq!(r.len(), 1, "same url -> same record, tags merged");
    }
}
