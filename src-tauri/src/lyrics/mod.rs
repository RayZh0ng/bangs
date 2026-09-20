//! Timed lyrics for whatever is playing. QQ Music's public lyric endpoint is
//! the source, because it is the one that matches what the player itself
//! shows; only the track title and artist leave the machine, and every result
//! is cached on disk so a track is looked up once.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::media::MediaState;
use crate::settings::SettingsState;

mod qrc;

const SEARCH_URL: &str = "https://c.y.qq.com/soso/fcgi-bin/client_search_cp";
const LYRIC_URL: &str = "https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg";
/// Returns QRC, which carries a timing per character.
const QRC_URL: &str = "https://c.y.qq.com/qqmusic/fcgi-bin/lyric_download.fcg";
const REFERER: &str = "https://y.qq.com/portal/player.html";
/// Search results to weigh. Wide enough that a track still turns up when the
/// artist term misleads the ranking and its album-mates come first.
const SEARCH_RESULTS: usize = 20;
/// How far a result's length may sit from the one the player reports and still
/// be taken for the same recording. Two masters of a song are a second or two
/// apart; a cover or a live take is tens of seconds.
const DURATION_SLACK: f64 = 10.0;
/// Bumped whenever the matching changes enough that what an older version
/// picked is not to be trusted — a cached file may hold a cover's timeline.
const CACHE_VERSION: u32 = 2;
const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricWord {
    /// Seconds into the track.
    pub at: f64,
    pub duration: f64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    /// Seconds into the track.
    pub at: f64,
    pub text: String,
    pub translation: Option<String>,
    /// Per-character timing, when the source has it (QRC).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<LyricWord>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    /// The track these lines belong to, so the webview can drop stale ones.
    pub track: String,
    pub lines: Vec<LyricLine>,
}

#[derive(Default)]
pub struct LyricsHub {
    current: Mutex<Lyrics>,
    requests: Mutex<Option<Sender<Request>>>,
}

impl LyricsHub {
    pub fn current(&self) -> Lyrics {
        self.current.lock().unwrap().clone()
    }
}

struct Request {
    track: String,
    title: String,
    artist: String,
    /// Track length in seconds, when the player reports one. The surest way to
    /// tell a recording from a cover of it.
    duration: Option<f64>,
}

/// Identifies a track across restarts; also the cache key.
pub fn track_key(media: &MediaState) -> String {
    format!("{}\u{1f}{}", media.title.trim(), media.artist.trim())
}

/// Called whenever the media state changes; fetches once per track.
pub fn sync(app: &AppHandle, media: Option<&MediaState>) {
    let hub = app.state::<LyricsHub>();
    let Some(media) = media.filter(|media| !media.title.is_empty()) else {
        publish(app, Lyrics::default());
        return;
    };

    let track = track_key(media);
    if hub.current().track == track {
        return;
    }
    // Clear immediately: the old lines belong to the previous song.
    publish(app, Lyrics { track: track.clone(), lines: Vec::new() });

    if !app.state::<SettingsState>().get().lyrics_enabled {
        return;
    }
    let request = Request {
        track,
        title: media.title.clone(),
        artist: media.artist.clone(),
        duration: media.duration,
    };
    let sender = hub.requests.lock().unwrap().clone();
    if let Some(sender) = sender {
        let _ = sender.send(request);
    }
}

/// Applies a change to the lyrics setting: clear the lines, or look up the
/// track that is playing right now.
pub fn set_enabled(app: &AppHandle, enabled: bool) {
    let media = app.state::<crate::media::MediaHub>().current();
    publish(app, Lyrics::default());
    if enabled {
        sync(app, media.as_ref());
    }
}

fn publish(app: &AppHandle, lyrics: Lyrics) {
    {
        let hub = app.state::<LyricsHub>();
        let mut current = hub.current.lock().unwrap();
        if *current == lyrics {
            return;
        }
        *current = lyrics.clone();
    }
    let _ = app.emit("bangs://lyrics", lyrics);
}

pub fn start(app: AppHandle) {
    let (sender, requests) = mpsc::channel();
    *app.state::<LyricsHub>().requests.lock().unwrap() = Some(sender);
    thread::spawn(move || run(app, requests));
}

fn run(app: AppHandle, requests: Receiver<Request>) {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map(|dir| dir.join("lyrics"))
        .ok();
    if let Some(dir) = &cache_dir {
        let _ = fs::create_dir_all(dir);
        sweep_old_cache(dir);
    }

    while let Ok(request) = requests.recv() {
        // Only the newest request matters; skip anything already queued behind it.
        let request = requests.try_iter().last().unwrap_or(request);
        let path = cache_dir.as_ref().map(|dir| dir.join(cache_name(&request.track)));

        let cached = path
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|raw| serde_json::from_str::<Vec<LyricLine>>(&raw).ok());

        let lines = match cached {
            Some(lines) => lines,
            None => {
                let lines =
                    fetch(&request.title, &request.artist, request.duration).unwrap_or_default();
                // Never cache a miss: the lookup may just have been offline.
                if !lines.is_empty() {
                    if let (Some(path), Ok(raw)) = (&path, serde_json::to_string(&lines)) {
                        let _ = fs::write(path, raw);
                    }
                }
                lines
            }
        };

        // A track change while the request was in flight wins.
        if app.state::<LyricsHub>().current().track == request.track {
            publish(&app, Lyrics { track: request.track, lines });
        }
    }
}

fn cache_name(track: &str) -> String {
    let mut hasher = DefaultHasher::new();
    track.hash(&mut hasher);
    format!("{CACHE_VERSION}-{:016x}.json", hasher.finish())
}

/// Throws away what an older version cached. Those files were matched by rules
/// this one no longer trusts, and a wrong lyric looks like a broken one.
fn sweep_old_cache(dir: &std::path::Path) {
    let prefix = format!("{CACHE_VERSION}-");
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".json") && !name.starts_with(&prefix) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn client() -> Option<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .user_agent("Mozilla/5.0")
        .build()
        .ok()
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn fetch(title: &str, artist: &str, duration: Option<f64>) -> Option<Vec<LyricLine>> {
    let client = client()?;
    // Spotify decorates its titles ("- Remastered 2011", "(feat. …)") and
    // lists every artist; the plain form is what a lyric search understands,
    // so it is the second thing to try.
    let plain_title = plain_title(title);
    let plain_artist = first_artist(artist);
    // `true` where only an exact title will do; see the last query.
    let mut queries = vec![(format!("{title} {artist}"), false)];
    push_query(&mut queries, format!("{plain_title} {plain_artist}"), false);
    // Apple Music hands over romanized names for part of its Chinese
    // catalogue — 赵雷 arrives as "Lei Zhao" — and an artist the search cannot
    // place drags the song itself out of the results, because every other
    // track on the album matches the query just as poorly. The title alone
    // finds it, and taking only an exact title keeps that from turning into
    // a different song with a similar name.
    push_query(&mut queries, plain_title.clone(), true);
    for (query, exact_title) in queries {
        if let Some(lines) =
            lookup(&client, &query, &plain_title, &plain_artist, duration, exact_title)
        {
            return Some(lines);
        }
    }
    None
}

/// Queues a query unless an earlier one already says the same thing.
fn push_query(queries: &mut Vec<(String, bool)>, query: String, exact_title: bool) {
    let query = query.trim().to_string();
    if query.is_empty() || queries.iter().any(|(queued, _)| *queued == query) {
        return;
    }
    queries.push((query, exact_title));
}

/// Drops the suffixes players add to a title: "Song - Live", "Song (feat. X)".
/// A title that is all decoration — "(Don't Fear) The Reaper" — keeps its own
/// text, because an empty one matches every song in the results.
fn plain_title(title: &str) -> String {
    let plain = title.split(" - ").next().unwrap_or(title);
    let plain = plain.split(['(', '（', '[']).next().unwrap_or(plain).trim();
    if plain.is_empty() { title.trim().to_string() } else { plain.to_string() }
}

/// The lead artist; a search does worse with the whole billing.
fn first_artist(artist: &str) -> String {
    let lead = artist.split([',', '&', '/', ';']).next().unwrap_or(artist).trim();
    if lead.is_empty() { artist.trim().to_string() } else { lead.to_string() }
}

fn lookup(
    client: &reqwest::blocking::Client,
    query: &str,
    title: &str,
    artist: &str,
    duration: Option<f64>,
    exact_title: bool,
) -> Option<Vec<LyricLine>> {
    let response = client
        .get(format!("{SEARCH_URL}?w={}&format=json&n={SEARCH_RESULTS}&p=1", encode(query)))
        .header("Referer", REFERER)
        .send();
    let body = match response.and_then(|response| response.text()) {
        Ok(body) => body,
        Err(error) => {
            eprintln!("[lyrics] search failed: {error}");
            return None;
        }
    };
    let search: serde_json::Value = match serde_json::from_str(strip_jsonp(&body)) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("[lyrics] search returned no JSON: {error}");
            return None;
        }
    };

    let songs = search["data"]["song"]["list"].as_array()?;
    let song = best_match(songs, title, artist, duration, exact_title)?;
    let song_mid = song["songmid"].as_str().unwrap_or_default().to_string();

    // Per-character timing first; the plain LRC endpoint is the fallback.
    if let Some(id) = song["songid"].as_i64() {
        if let Some(lines) = fetch_qrc(client, id) {
            return Some(lines);
        }
    }
    if song_mid.is_empty() {
        return None;
    }
    // The endpoint answers -1901 now and then; a second ask usually works.
    for attempt in 0..2 {
        if attempt > 0 {
            thread::sleep(Duration::from_millis(400));
        }
        let Some(payload) = lyric_payload(client, &song_mid) else { continue };
        let lines = parse_lrc(payload["lyric"].as_str().unwrap_or_default());
        if lines.is_empty() {
            continue;
        }
        return Some(with_translation(
            lines,
            parse_lrc(payload["trans"].as_str().unwrap_or_default()),
        ));
    }
    None
}

fn lyric_payload(client: &reqwest::blocking::Client, song_mid: &str) -> Option<serde_json::Value> {
    let raw = client
        .get(format!(
            "{LYRIC_URL}?songmid={}&format=json&nobase64=1&g_tk=5381",
            encode(song_mid)
        ))
        .header("Referer", REFERER)
        .send()
        .ok()?
        .text()
        .ok()?;
    serde_json::from_str(strip_jsonp(&raw)).ok()
}

/// Titles and names are compared folded: catalogues disagree about traditional
/// and simplified characters — Apple Music says 當時的月亮 where QQ Music says
/// 当时的月亮 — and about spacing and case.
fn normalize(value: &str) -> String {
    let folded = crate::platform::to_simplified(value);
    folded.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase()
}

/// Prefers an exact title match by the same artist; the search endpoint
/// otherwise happily returns covers and remixes first.
fn best_match<'a>(
    songs: &'a [serde_json::Value],
    title: &str,
    artist: &str,
    duration: Option<f64>,
    exact_title: bool,
) -> Option<&'a serde_json::Value> {
    let wanted_title = normalize(title);
    let wanted_artist = normalize(artist);

    let score = |song: &serde_json::Value| {
        let name = normalize(song["songname"].as_str().unwrap_or_default());
        let singers = song["singer"]
            .as_array()
            .map(|singers| {
                singers
                    .iter()
                    .map(|singer| normalize(singer["name"].as_str().unwrap_or_default()))
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .unwrap_or_default();
        let title_score = if name == wanted_title {
            2
        } else if name.contains(&wanted_title) || wanted_title.contains(&name) {
            1
        } else {
            0
        };
        let artist_score = if wanted_artist.is_empty() {
            0
        } else if singers.contains(&wanted_artist) || wanted_artist.contains(&singers) {
            2
        } else {
            0
        };
        (title_score, artist_score)
    };

    // How far this recording is from the one playing, when both lengths are
    // known. An unknown one is worth nothing and loses to any known match.
    let gap = |song: &serde_json::Value| match (duration, song["interval"].as_f64()) {
        (Some(wanted), Some(interval)) if wanted > 0.0 && interval > 0.0 => {
            Some((interval - wanted).abs())
        }
        _ => None,
    };

    let mut best: Option<(&serde_json::Value, i32, f64)> = None;
    for song in songs {
        let (title_score, artist_score) = score(song);
        if title_score == 0 || (exact_title && title_score < 2) {
            continue;
        }
        // A recording of a different length is a different recording. A cover
        // carries the same words on a timeline of its own, which reads as
        // lyrics that drift — worse than no lyrics at all.
        let gap = gap(song);
        if gap.is_some_and(|gap| gap > DURATION_SLACK) {
            continue;
        }
        let total = title_score + artist_score;
        let gap = gap.unwrap_or(f64::MAX);
        // The endpoint has already ranked what it returned, so a later song
        // has to beat the one in hand outright — by scoring higher, or by
        // being the length the player is playing.
        let better = best.is_none_or(|(_, best_total, best_gap)| {
            total > best_total || (total == best_total && gap < best_gap)
        });
        if better {
            best = Some((song, total, gap));
        }
    }
    best.map(|(song, _, _)| song)
}

/// Downloads the QRC document and turns it into timed lines. The response is
/// XML whose CDATA sections hold the encrypted lyric and its translation.
fn fetch_qrc(client: &reqwest::blocking::Client, song_id: i64) -> Option<Vec<LyricLine>> {
    let document = client
        .get(format!(
            "{QRC_URL}?version=15&miniversion=82&lrctype=4&musicid={song_id}"
        ))
        .header("Referer", "https://y.qq.com")
        .send()
        .ok()?
        .text()
        .ok()?;

    let blocks: Vec<&str> = document
        .split("<![CDATA[")
        .skip(1)
        .filter_map(|block| block.split("]]>").next())
        .map(str::trim)
        .collect();

    let lines = parse_qrc(&qrc::decrypt_lyrics(blocks.first()?)?);
    if lines.is_empty() {
        return None;
    }
    let translations = blocks
        .get(1)
        .filter(|block| !block.is_empty())
        .and_then(|block| qrc::decrypt_lyrics(block))
        .map(|text| parse_lrc(&strip_qrc_timings(&text)))
        .unwrap_or_default();
    Some(with_translation(lines, translations))
}

/// The lyric lives in the `LyricContent` attribute of the QRC document.
fn qrc_content(document: &str) -> Option<&str> {
    let start = document.find("LyricContent=\"")? + "LyricContent=\"".len();
    let rest = &document[start..];
    let end = rest.rfind("\"")?;
    Some(&rest[..end])
}

/// `[start,duration]字(start,duration)字(start,duration)…`, milliseconds.
fn parse_qrc(document: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    for row in qrc_content(document).unwrap_or_default().lines() {
        let Some(rest) = row.strip_prefix('[') else { continue };
        let Some((head, body)) = rest.split_once(']') else { continue };
        let Some((start, _)) = head.split_once(',') else { continue };
        let Ok(start): Result<f64, _> = start.trim().parse() else { continue };

        let words = parse_qrc_words(body);
        let text: String = words.iter().map(|word| word.text.as_str()).collect();
        if text.trim().is_empty() {
            continue;
        }
        lines.push(LyricLine {
            at: words.first().map(|word| word.at).unwrap_or(start / 1000.0),
            text,
            translation: None,
            words,
        });
    }
    lines.sort_by(|a, b| a.at.total_cmp(&b.at));
    lines
}

fn parse_qrc_words(body: &str) -> Vec<LyricWord> {
    let mut words = Vec::new();
    let mut text = String::new();
    let mut chars = body.chars().peekable();

    while let Some(character) = chars.next() {
        if character != '(' {
            text.push(character);
            continue;
        }
        let timing: String = chars.by_ref().take_while(|next| *next != ')').collect();
        // A literal bracket in the lyric is not a timing group.
        let Some((at, duration)) = timing
            .split_once(',')
            .and_then(|(at, duration)| Some((at.trim().parse::<f64>().ok()?, duration.trim().parse::<f64>().ok()?)))
        else {
            text.push('(');
            text.push_str(&timing);
            text.push(')');
            continue;
        };
        words.push(LyricWord {
            at: at / 1000.0,
            duration: duration / 1000.0,
            text: std::mem::take(&mut text),
        });
    }
    if !text.is_empty() {
        if let Some(last) = words.last_mut() {
            last.text.push_str(&text);
        }
    }
    words
}

/// Translations come as QRC too; drop the per-character stamps to read them as LRC.
fn strip_qrc_timings(document: &str) -> String {
    let content = qrc_content(document).unwrap_or(document);
    let mut cleaned = String::with_capacity(content.len());
    let mut depth = 0;
    for character in content.chars() {
        match character {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => cleaned.push(character),
            _ => {}
        }
    }
    cleaned
}

/// Some responses come back wrapped in a callback, e.g. `MusicJsonCallback({…})`.
fn strip_jsonp(raw: &str) -> &str {
    match (raw.find('{'), raw.rfind('}')) {
        (Some(start), Some(end)) if end > start => &raw[start..=end],
        _ => raw,
    }
}

/// `[mm:ss.xx]text`, with repeated stamps for shared lines.
fn parse_lrc(raw: &str) -> Vec<LyricLine> {
    let mut lines: Vec<LyricLine> = Vec::new();
    for row in raw.lines() {
        let mut rest = row;
        let mut stamps = Vec::new();
        while let Some(close) = rest.strip_prefix('[').and_then(|rest| rest.find(']')) {
            let stamp = &rest[1..=close];
            if let Some(at) = parse_stamp(stamp) {
                stamps.push(at);
            }
            rest = &rest[close + 2..];
        }
        let text = rest.trim();
        if text.is_empty() {
            continue;
        }
        for at in stamps {
            lines.push(LyricLine { at, text: text.to_string(), translation: None, words: Vec::new() });
        }
    }
    lines.sort_by(|a, b| a.at.total_cmp(&b.at));
    lines
}

fn parse_stamp(stamp: &str) -> Option<f64> {
    let (minutes, rest) = stamp.split_once(':')?;
    let minutes: f64 = minutes.trim().parse().ok()?;
    let seconds: f64 = rest.replace(':', ".").parse().ok()?;
    Some(minutes * 60.0 + seconds)
}

/// Pairs each line with the translation carrying the same timestamp.
fn with_translation(mut lines: Vec<LyricLine>, translations: Vec<LyricLine>) -> Vec<LyricLine> {
    for line in &mut lines {
        line.translation = translations
            .iter()
            .find(|translation| (translation.at - line.at).abs() < 0.05)
            .map(|translation| translation.text.clone())
            .filter(|text| !text.is_empty() && *text != line.text);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(name: &str, singers: &[&str]) -> serde_json::Value {
        serde_json::json!({
            "songname": name,
            "singer": singers.iter().map(|name| serde_json::json!({ "name": name })).collect::<Vec<_>>(),
        })
    }

    fn song_of(name: &str, singers: &[&str], interval: i64) -> serde_json::Value {
        let mut song = song(name, singers);
        song["interval"] = serde_json::json!(interval);
        song
    }

    fn pick<'a>(songs: &'a [serde_json::Value], title: &str, artist: &str, exact: bool) -> Option<&'a str> {
        best_match(songs, title, artist, None, exact).map(|song| song["songname"].as_str().unwrap())
    }

    fn pick_lasting<'a>(
        songs: &'a [serde_json::Value],
        title: &str,
        artist: &str,
        duration: f64,
    ) -> Option<&'a str> {
        best_match(songs, title, artist, Some(duration), false)
            .map(|song| song["songname"].as_str().unwrap())
    }

    #[test]
    fn keeps_the_best_ranked_of_equally_good_matches() {
        // A duet scores exactly like the original, and the endpoint put the
        // original first.
        let songs = [song("无法长大", &["赵雷"]), song("无法长大", &["张大为", "赵雷"])];
        assert_eq!(pick(&songs, "无法长大", "赵雷", false), Some("无法长大"));
        assert!(std::ptr::eq(best_match(&songs, "无法长大", "赵雷", None, false).unwrap(), &songs[0]));
    }

    #[test]
    fn ignores_other_songs_by_the_same_artist() {
        let songs = [song("鼓楼", &["赵雷"]), song("无法长大", &["赵雷"])];
        assert_eq!(pick(&songs, "无法长大", "赵雷", false), Some("无法长大"));
    }

    #[test]
    fn finds_the_track_when_the_artist_arrives_romanized() {
        // Apple Music says "Lei Zhao"; QQ Music says 赵雷. Only the title lines up.
        let songs = [song("鼓楼", &["赵雷"]), song("无法长大", &["赵雷"])];
        assert_eq!(pick(&songs, "无法长大", "Lei Zhao", true), Some("无法长大"));
    }

    #[test]
    fn a_title_only_search_turns_down_an_inexact_title() {
        let songs = [song("无法长大 (DJ 阿若版)", &["赵雷"])];
        assert_eq!(pick(&songs, "无法长大", "Lei Zhao", true), None);
        assert_eq!(pick(&songs, "无法长大", "Lei Zhao", false), Some("无法长大 (DJ 阿若版)"));
    }

    #[test]
    fn reads_past_traditional_characters() {
        // Apple Music hands over 當時的月亮; QQ Music lists 当时的月亮.
        let songs = [song("当时的月亮", &["王菲"])];
        assert_eq!(pick(&songs, "當時的月亮", "Faye Wong", true), Some("当时的月亮"));
    }

    #[test]
    fn turns_down_a_cover_of_the_right_length_elsewhere() {
        // 小宇: the cover the search ranks first runs 269s, the recording
        // playing runs 228s, and their words sit on different timelines.
        let songs = [song_of("小宇", &["蓝心羽"], 269), song_of("小宇", &["张震岳"], 227)];
        assert_eq!(pick_lasting(&songs, "小宇", "A-Yue Chang", 228.0), Some("小宇"));
        assert!(std::ptr::eq(
            best_match(&songs, "小宇", "A-Yue Chang", Some(228.0), false).unwrap(),
            &songs[1],
        ));
    }

    #[test]
    fn keeps_a_result_whose_length_is_unknown() {
        let songs = [song("小宇", &["张震岳"])];
        assert_eq!(pick_lasting(&songs, "小宇", "A-Yue Chang", 228.0), Some("小宇"));
    }

    #[test]
    fn queues_each_query_once() {
        let mut queries = vec![("Song Artist".to_string(), false)];
        push_query(&mut queries, "Song Artist".to_string(), false);
        push_query(&mut queries, "Song ".to_string(), true);
        push_query(&mut queries, "Song".to_string(), true);
        assert_eq!(queries, [("Song Artist".to_string(), false), ("Song".to_string(), true)]);
    }
}
