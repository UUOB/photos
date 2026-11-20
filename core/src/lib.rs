//! Photos without its screens: what the Android, Mac and Linux windows share.
//! The data is UUOB's (through the SDK, `uuob-client`); these are the rules
//! Photos applies to it, as the web version did (`app/Photos.svelte`):
//!
//! - One container per month, named `YYYY-MM` and created with the key
//!   `month<YYYYMM>00000`, so the same month started on two devices is one
//!   container.
//! - A photo is a record in its month's `photos` collection:
//!   `{ taken, added, name, type, size, file, w?, h?, thumb?, duration?, deleted? }`.
//!   `thumb` is a small JPEG inside the record, so the grid works offline;
//!   the original is a file.
//! - Deleting sets `deleted`; after 30 days the photo is removed for good.
//!
//! Records, lists and answers are JSON text, as the SDK gives them.

uniffi::setup_scaffolding!();

/// UUOB's SDK, in the same library, so Kotlin and Swift get both.
pub use uuob_client;

use base64::Engine;
use jiff::{Timestamp, tz::TimeZone};
use serde_json::{Map, Value as Json, json};

/// How long a deleted photo stays in Recently Deleted.
pub const TRASH_DAYS: i64 = 30;
const DAY: i64 = 86_400_000;
/// A preview's shorter side, at most.
const THUMB: f64 = 240.0;

fn parse(s: &str) -> Json {
    serde_json::from_str(s).unwrap_or(Json::Null)
}

fn records(s: &str) -> Vec<Json> {
    match parse(s) {
        Json::Array(a) => a,
        _ => vec![],
    }
}

fn num(r: &Json, k: &str) -> f64 {
    r[k].as_f64().unwrap_or(0.0)
}

/// Photos' version.
#[uniffi::export]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").into()
}

fn zoned(ms: i64) -> jiff::Zoned {
    Timestamp::from_millisecond(ms).unwrap_or(Timestamp::UNIX_EPOCH).to_zoned(TimeZone::system())
}

/// The month a time falls in, on this device's clock: `2026-11`.
#[uniffi::export]
pub fn month_of(ms: i64) -> String {
    let z = zoned(ms);
    format!("{:04}-{:02}", z.year(), z.month())
}

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/// `2026-11` as people read it: "November 2026".
#[uniffi::export]
pub fn month_label(month: String) -> String {
    let (y, m) = month.split_once('-').unwrap_or(("", ""));
    let m: usize = m.parse().unwrap_or(1);
    format!("{} {y}", MONTHS[m.clamp(1, 12) - 1])
}

/// The key a month's container is created with, the same on every device.
#[uniffi::export]
pub fn month_container_key(month: String) -> String {
    format!("month{}00000", month.replace('-', ""))
}

/// When a photo was taken: "Nov 19, 2026, 9:30 AM".
#[uniffi::export]
pub fn when_label(ms: i64) -> String {
    let z = zoned(ms);
    let h = z.hour() as i32;
    format!("{} {}, {}, {}:{:02} {}", &MONTHS[z.month() as usize - 1][..3], z.day(), z.year(), (h + 11) % 12 + 1, z.minute(), if h < 12 { "AM" } else { "PM" })
}

/// A video's length: "1:05".
#[uniffi::export]
pub fn duration_label(seconds: f64) -> String {
    // As the web version wrote it (seconds rounded, never carried into minutes).
    format!("{}:{:02}", (seconds / 60.0).floor() as i64, (seconds % 60.0).round() as i64)
}

/// How many days a deleted photo has left, at least 1.
#[uniffi::export]
pub fn days_left(deleted: i64, now: i64) -> i64 {
    let left = deleted + TRASH_DAYS * DAY - now;
    ((left as f64 / DAY as f64).ceil() as i64).max(1)
}

pub fn is_video(r: &Json) -> bool {
    r["type"].as_str().is_some_and(|t| t.starts_with("video/"))
}

/// The grid: the photos (or, with `trash`, the deleted ones) grouped by the
/// month they were taken, newest first, plus how many there are of each:
/// `{ groups: [{ month, label, items: [...] }], photos, trash }`.
#[uniffi::export]
pub fn gallery(records_json: String, trash: bool) -> String {
    gallery_of(&records(&records_json), trash).to_string()
}

pub fn gallery_of(all: &[Json], trash: bool) -> Json {
    let mut photos: Vec<&Json> = all.iter().filter(|p| p["deleted"].is_null()).collect();
    let mut deleted: Vec<&Json> = all.iter().filter(|p| !p["deleted"].is_null()).collect();
    photos.sort_by(|a, b| num(b, "taken").total_cmp(&num(a, "taken")));
    deleted.sort_by(|a, b| num(b, "deleted").total_cmp(&num(a, "deleted")));
    let shown = if trash { &deleted } else { &photos };
    let mut groups: Vec<(String, Vec<Json>)> = vec![];
    for p in shown {
        let key = month_of(num(p, "taken") as i64);
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, items)) => items.push((*p).clone()),
            None => groups.push((key, vec![(*p).clone()])),
        }
    }
    let groups: Vec<Json> = groups.into_iter().map(|(month, items)| json!({ "label": month_label(month.clone()), "month": month, "items": items })).collect();
    json!({ "groups": groups, "photos": photos.len(), "trash": deleted.len() })
}

/// The photos deleted so long ago that they go for good now.
#[uniffi::export]
pub fn expired(records_json: String, now: i64) -> String {
    let out: Vec<Json> = records(&records_json).into_iter().filter(|p| p["deleted"].as_i64().is_some_and(|d| now - d > TRASH_DAYS * DAY)).collect();
    Json::Array(out).to_string()
}

/// A record as it is saved: without the `container` the SDK adds when it
/// lists records across containers.
#[uniffi::export]
pub fn stored(record: String) -> String {
    stored_of(parse(&record)).to_string()
}

pub fn stored_of(mut r: Json) -> Json {
    if let Some(o) = r.as_object_mut() {
        o.remove("container");
    }
    r
}

/// The record moved to Recently Deleted (`deleted` set) or back (`now` 0).
#[uniffi::export]
pub fn mark_deleted(record: String, now: i64) -> String {
    let mut r = stored_of(parse(&record));
    if let Some(o) = r.as_object_mut() {
        if now > 0 {
            o.insert("deleted".into(), json!(now));
        } else {
            o.remove("deleted");
        }
    }
    r.to_string()
}

/// A new photo's record. `preview`: `{ w, h, thumb, duration }` as far as
/// one could be made (`{}` otherwise).
#[uniffi::export]
pub fn new_record(taken: i64, added: i64, name: String, content_type: String, size: i64, file: String, preview: String) -> String {
    let mut r = Map::new();
    r.insert("taken".into(), json!(taken));
    r.insert("added".into(), json!(added));
    r.insert("name".into(), json!(name));
    r.insert("type".into(), json!(content_type));
    r.insert("size".into(), json!(size));
    r.insert("file".into(), json!(file));
    if let Json::Object(p) = parse(&preview) {
        for k in ["w", "h", "duration", "thumb"] {
            if let Some(v) = p.get(k).filter(|v| !v.is_null()) {
                r.insert(k.into(), v.clone());
            }
        }
    }
    Json::Object(r).to_string()
}

/// The size of a preview for a picture of `w`×`h`: the shorter side 240 at
/// most, never larger than the picture.
#[uniffi::export]
pub fn thumb_size(w: u32, h: u32) -> Vec<u32> {
    let scale = (THUMB / w.min(h).max(1) as f64).min(1.0);
    vec![((w as f64 * scale).round() as u32).max(1), ((h as f64 * scale).round() as u32).max(1)]
}

/// A JPEG as the `thumb` field keeps it: a data URL.
#[uniffi::export]
pub fn jpeg_data_url(jpeg: Vec<u8>) -> String {
    format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(jpeg))
}

/// The JPEG inside a `thumb` data URL (empty if it isn't one).
#[uniffi::export]
pub fn thumb_bytes(thumb: String) -> Vec<u8> {
    thumb.split_once(";base64,").and_then(|(_, b)| base64::engine::general_purpose::STANDARD.decode(b).ok()).unwrap_or_default()
}

/// What a newly picked file is taken as: its own time, or now.
#[uniffi::export]
pub fn taken_or_now(modified: i64, now: i64) -> i64 {
    if modified > 0 { modified } else { now }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn months_and_their_containers() {
        assert_eq!(month_label("2026-11".into()), "November 2026");
        assert_eq!(month_container_key("2026-11".into()), "month20261100000");
    }

    #[test]
    fn previews_are_at_most_240_on_the_short_side() {
        assert_eq!(thumb_size(4000, 3000), vec![320, 240]);
        assert_eq!(thumb_size(1080, 1920), vec![240, 427]);
        assert_eq!(thumb_size(100, 50), vec![100, 50], "never larger");
    }

    #[test]
    fn video_lengths_read_like_the_web_version() {
        assert_eq!(duration_label(65.2), "1:05");
        assert_eq!(duration_label(9.0), "0:09");
        assert_eq!(duration_label(59.7), "0:60"); // as Math.round wrote it
    }

    #[test]
    fn trash_counts_days_and_expires_after_30() {
        let now = 1_800_000_000_000;
        assert_eq!(days_left(now, now), 30);
        assert_eq!(days_left(now - 29 * DAY - 1, now), 1);
        assert_eq!(days_left(now - 40 * DAY, now), 1);
        let all = json!([{ "id": "a", "deleted": now - 31 * DAY }, { "id": "b", "deleted": now - DAY }, { "id": "c" }]).to_string();
        let gone: Vec<Json> = serde_json::from_str(&expired(all, now)).unwrap();
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0]["id"], "a");
    }

    #[test]
    fn the_gallery_groups_by_month_newest_first() {
        let day = |d: i64| 1_793_000_000_000 + d * DAY; // in Oct/Nov 2026
        let all = json!([
            { "id": "old", "taken": day(0) },
            { "id": "new", "taken": day(30) },
            { "id": "mid", "taken": day(29) },
            { "id": "gone", "taken": day(1), "deleted": day(31) },
        ]);
        let g = gallery_of(all.as_array().unwrap(), false);
        assert_eq!((g["photos"].clone(), g["trash"].clone()), (json!(3), json!(1)));
        let ids: Vec<&str> = g["groups"].as_array().unwrap().iter().flat_map(|g| g["items"].as_array().unwrap()).map(|p| p["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["new", "mid", "old"]);
        let t = gallery_of(all.as_array().unwrap(), true);
        assert_eq!(t["groups"][0]["items"][0]["id"], "gone");
    }

    #[test]
    fn records_keep_the_web_versions_shape() {
        let r: Json = serde_json::from_str(&new_record(5, 6, "a.jpg".into(), "image/jpeg".into(), 7, "f1".into(), r#"{"w":4,"h":3,"thumb":"data:x"}"#.into())).unwrap();
        assert_eq!(r, json!({ "taken": 5, "added": 6, "name": "a.jpg", "type": "image/jpeg", "size": 7, "file": "f1", "w": 4, "h": 3, "thumb": "data:x" }));
        let r: Json = serde_json::from_str(&new_record(5, 6, "b.heic".into(), "image/heic".into(), 7, "f2".into(), "{}".into())).unwrap();
        assert!(r.get("thumb").is_none(), "no preview: stored without one");
        let d: Json = serde_json::from_str(&mark_deleted(json!({ "id": "x", "container": "m", "taken": 1 }).to_string(), 9)).unwrap();
        assert_eq!(d, json!({ "id": "x", "taken": 1, "deleted": 9 }));
        let back: Json = serde_json::from_str(&mark_deleted(d.to_string(), 0)).unwrap();
        assert_eq!(back, json!({ "id": "x", "taken": 1 }));
        assert_eq!(thumb_bytes(jpeg_data_url(vec![1, 2, 3])), vec![1, 2, 3]);
    }
}
