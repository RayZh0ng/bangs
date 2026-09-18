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

const SEARCH_URL: &str = "https://c.y.qq.com/soso/fcgi-bin/client_search_cp";
const LYRIC_URL: &str = "https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg";
const REFERER: &str = "https://y.qq.com/portal/player.html";
const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    /// Seconds into the track.
    pub at: f64,
    pub text: String,
    pub translation: Option<String>,
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
                let lines = fetch(&request.title, &request.artist).unwrap_or_default();
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
    format!("{:016x}.json", hasher.finish())
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

fn fetch(title: &str, artist: &str) -> Option<Vec<LyricLine>> {
    let client = client()?;
    let query = format!("{title} {artist}");
    let response = client
        .get(format!("{SEARCH_URL}?w={}&format=json&n=5&p=1", encode(&query)))
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
    let song_mid = best_match(songs, title, artist)?;
    // The endpoint answers -1901 now and then; a second ask usually works.
    for attempt in 0..2 {
        if attempt > 0 {
            thread::sleep(Duration::from_millis(400));
        }
        let Some(payload) = lyric_payload(&client, &song_mid) else { continue };
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

fn normalize(value: &str) -> String {
    value.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase()
}

/// Prefers an exact title match by the same artist; the search endpoint
/// otherwise happily returns covers and remixes first.
fn best_match(songs: &[serde_json::Value], title: &str, artist: &str) -> Option<String> {
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
        title_score + artist_score
    };

    songs
        .iter()
        .max_by_key(|song| score(song))
        .filter(|song| score(song) > 0)
        .and_then(|song| song["songmid"].as_str())
        .map(str::to_string)
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
            lines.push(LyricLine { at, text: text.to_string(), translation: None });
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
