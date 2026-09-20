//! Lyrics for whatever is playing. The active player selects a provider order;
//! only track metadata leaves the machine, and successful results are cached
//! on disk so a track is looked up once per source.

use std::collections::{hash_map::DefaultHasher, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

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
    /// Whether the accepted lines have real timestamps.
    #[serde(default)]
    pub timed: bool,
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
    album: String,
    duration: Option<f64>,
    player: PlayerKind,
    /// Whether a translated line is worth extra looking; the tray decides.
    translate: bool,
    attempt: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerKind {
    QqMusic,
    NetEase,
    Spotify,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Provider {
    QqQrc,
    QqLrc,
    NetEase,
}

#[derive(Debug, Clone)]
struct FetchedLyrics {
    provider: Provider,
    timed: bool,
    lines: Vec<LyricLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheFile {
    version: u8,
    provider: String,
    timed: bool,
    lines: Vec<LyricLine>,
}

/// Search results to weigh. Wide enough that a track still turns up when the
/// artist term misleads the ranking and its album-mates come first.
const SEARCH_RESULTS: usize = 20;
/// How far a result's length may sit from the one the player reports and still
/// be taken for the same recording. Two masters of a song are a second or two
/// apart; a cover or a live take is tens of seconds.
const DURATION_SLACK: f64 = 10.0;
/// Near enough to be the same master, not another take of the same song.
const SAME_RECORDING: f64 = 3.0;
/// How far down the results a length alone is still worth trusting. The query
/// carried the title and the artist, so the engine put what it thinks answers
/// them at the top; further down, a length that happens to line up is chance.
const TRUSTED_RANK: usize = 5;
/// Results asked for a lyric before a query is written off. One that comes
/// back wordless is not the end of it, but the list is ordered by how likely
/// each is to be the song, and the tail is not worth the round trips.
const ATTEMPTS: usize = 3;
/// Bumped whenever the matching changes enough that what an older version
/// picked is not to be trusted — a cached file may hold a cover's timeline.
const CACHE_VERSION: u8 = 5;
const RETRIES: u8 = 2;
const RETRY_DELAYS: [Duration; 2] = [Duration::from_secs(30), Duration::from_secs(120)];

/// Stable song identity; dynamic source, duration and artwork do not belong in it.
pub fn track_key(media: &MediaState) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}",
        normalize(&media.title),
        normalize(&media.artist),
        normalize(&media.album),
    )
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
    publish(
        app,
        Lyrics {
            track: track.clone(),
            timed: false,
            lines: Vec::new(),
        },
    );

    let settings = app.state::<SettingsState>().get();
    if !settings.lyrics_enabled {
        return;
    }
    let request = Request {
        track,
        title: media.title.clone(),
        artist: media.artist.clone(),
        album: media.album.clone(),
        duration: media.duration,
        player: player_kind(media),
        translate: settings.lyrics_translation_enabled,
        attempt: 0,
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

    let mut delayed: VecDeque<(Instant, Request)> = VecDeque::new();
    loop {
        if let Some((when, _)) = delayed.front() {
            if *when <= Instant::now() {
                let (_, request) = delayed.pop_front().expect("retry queue front");
                if let Some(retry) = process_request(&app, request, &cache_dir) {
                    delayed.push_back(retry);
                    delayed.make_contiguous().sort_by_key(|(when, _)| *when);
                }
                continue;
            }
        }
        let timeout = delayed
            .front()
            .map(|(when, _)| when.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::from_secs(3600));
        match requests.recv_timeout(timeout) {
            Ok(first) => {
                // Only the newest request matters; skip anything already queued behind it.
                let request = requests.try_iter().last().unwrap_or(first);
                if let Some(retry) = process_request(&app, request, &cache_dir) {
                    delayed.push_back(retry);
                    delayed.make_contiguous().sort_by_key(|(when, _)| *when);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn process_request(
    app: &AppHandle,
    request: Request,
    cache_dir: &Option<std::path::PathBuf>,
) -> Option<(Instant, Request)> {
    if app.state::<LyricsHub>().current().track != request.track {
        return None;
    }
    let path = cache_dir
        .as_ref()
        .map(|dir| dir.join(cache_name(&request.track)));
    let fetched = path
        .as_ref()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|raw| read_cache(&raw))
        .or_else(|| {
            let fetched = fetch(
                &request.title,
                &request.artist,
                &request.album,
                request.duration,
                request.player,
                request.translate,
            );
            if let (Some(path), Some(fetched)) = (&path, fetched.as_ref()) {
                let cache = CacheFile {
                    version: CACHE_VERSION,
                    provider: fetched.provider.name().to_string(),
                    timed: fetched.timed,
                    lines: fetched.lines.clone(),
                };
                if let Ok(raw) = serde_json::to_string(&cache) {
                    let _ = fs::write(path, raw);
                }
            }
            fetched
        });

    if let Some(fetched) = fetched {
        if app.state::<LyricsHub>().current().track == request.track {
            publish(
                app,
                Lyrics {
                    track: request.track,
                    timed: fetched.timed,
                    lines: fetched.lines,
                },
            );
        }
        return None;
    }
    if request.attempt < RETRIES {
        let attempt = request.attempt;
        return Some((
            Instant::now() + RETRY_DELAYS[attempt as usize],
            Request {
                attempt: attempt + 1,
                ..request
            },
        ));
    }
    if app.state::<LyricsHub>().current().track == request.track {
        publish(
            app,
            Lyrics {
                track: request.track,
                timed: false,
                lines: Vec::new(),
            },
        );
    }
    None
}

fn cache_name(track: &str) -> String {
    let mut hasher = DefaultHasher::new();
    track.hash(&mut hasher);
    format!("{:016x}.json", hasher.finish())
}

/// Whether any line came with a translation under it.
fn has_translation(lines: &[LyricLine]) -> bool {
    lines
        .iter()
        .any(|line| line.translation.as_deref().is_some_and(|text| !text.trim().is_empty()))
}

/// Whether a translation would say anything this lyric does not. A lyric
/// already written in Han characters is the one the panel reads; Hangul, kana
/// and Latin words are what a translation is for.
fn wants_translation(lines: &[LyricLine]) -> bool {
    let (han, letters) = lines
        .iter()
        .flat_map(|line| line.text.chars())
        .filter(|character| character.is_alphabetic())
        .fold((0usize, 0usize), |(han, letters), character| {
            let is_han = matches!(character, '\u{3400}'..='\u{9fff}' | '\u{f900}'..='\u{faff}');
            (han + usize::from(is_han), letters + 1)
        });
    letters > 0 && han * 2 < letters
}

fn acceptable(result: &FetchedLyrics) -> bool {
    result.timed && has_words(&result.lines)
}

/// The lines a result opens with that are not the song: who wrote it, who
/// mixed it, or the standing sentence a catalogue returns for a track that has
/// no words at all.
const CREDITS: &[&str] = &[
    "作词", "作曲", "编曲", "制作", "混音", "监制", "出品", "录音", "母带", "和声", "词：", "曲：",
    "Written by", "Composed by", "Lyrics by", "Produced by", "Arranged by", "Mixed by",
];

/// Whether a result has anything to sing. An instrumental comes back as a
/// single standing sentence, and a credits-only lyric says as little — both
/// are better passed over so the next candidate, or the next provider, gets a
/// turn.
fn has_words(lines: &[LyricLine]) -> bool {
    lines.iter().any(|line| {
        let text = line.text.trim();
        !text.is_empty()
            && !text.contains("纯音乐")
            && !CREDITS.iter().any(|credit| text.starts_with(credit))
    })
}

fn has_timing(lines: &[LyricLine]) -> bool {
    !lines.is_empty()
        && lines
            .iter()
            .all(|line| line.at.is_finite() && line.at >= 0.0)
}

fn read_cache(raw: &str) -> Option<FetchedLyrics> {
    let cache: CacheFile = serde_json::from_str(raw).ok()?;
    if cache.version != CACHE_VERSION {
        return None;
    }
    let result = FetchedLyrics {
        provider: provider_from_name(&cache.provider)?,
        timed: cache.timed,
        lines: cache.lines,
    };
    acceptable(&result).then_some(result)
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

fn player_kind(media: &MediaState) -> PlayerKind {
    let haystack = format!("{} {}", media.source_id, media.app_name).to_lowercase();
    if haystack.contains("spotify") {
        PlayerKind::Spotify
    } else if haystack.contains("qqmusic")
        || haystack.contains("qq 音乐")
        || haystack.contains("qq音乐")
    {
        PlayerKind::QqMusic
    } else if haystack.contains("cloudmusic")
        || haystack.contains("netease")
        || haystack.contains("网易云")
        || haystack.contains("163music")
    {
        PlayerKind::NetEase
    } else {
        PlayerKind::Other
    }
}

fn provider_order(player: PlayerKind) -> &'static [Provider] {
    match player {
        PlayerKind::QqMusic => &[Provider::QqQrc, Provider::QqLrc, Provider::NetEase],
        PlayerKind::NetEase => &[Provider::NetEase, Provider::QqQrc, Provider::QqLrc],
        PlayerKind::Spotify | PlayerKind::Other => {
            &[Provider::QqQrc, Provider::NetEase, Provider::QqLrc]
        }
    }
}

impl Provider {
    fn name(self) -> &'static str {
        match self {
            Provider::QqQrc => "qq-qrc",
            Provider::QqLrc => "qq-lrc",
            Provider::NetEase => "netease",
        }
    }
}

fn provider_from_name(name: &str) -> Option<Provider> {
    match name {
        "qq-qrc" => Some(Provider::QqQrc),
        "qq-lrc" => Some(Provider::QqLrc),
        "netease" => Some(Provider::NetEase),
        _ => None,
    }
}

fn fetch(
    title: &str,
    artist: &str,
    album: &str,
    duration: Option<f64>,
    player: PlayerKind,
    translate: bool,
) -> Option<FetchedLyrics> {
    let client = client()?;
    let mut qq_cache: Option<Vec<serde_json::Value>> = None;
    // The first source with words wins, unless a translated line was asked
    // for and it has none: QQ carries no Chinese under a Korean lyric where
    // NetEase does, and stopping at the first answer would never find it.
    let mut untranslated: Option<FetchedLyrics> = None;
    for provider in provider_order(player) {
        let fetched = match provider {
            Provider::QqQrc | Provider::QqLrc => {
                let songs =
                    qq_cache.get_or_insert_with(|| qq_songs(&client, title, artist, duration));
                songs.iter().find_map(|song| {
                    let fetched = match provider {
                        Provider::QqQrc => fetch_qq_qrc_song(&client, song),
                        _ => fetch_qq_lrc_song(&client, song),
                    };
                    fetched.filter(acceptable)
                })
            }
            Provider::NetEase => fetch_netease(&client, title, artist, album, duration),
        };
        let Some(fetched) = fetched.filter(acceptable) else { continue };
        if !translate || has_translation(&fetched.lines) || !wants_translation(&fetched.lines) {
            return Some(fetched);
        }
        untranslated.get_or_insert(fetched);
    }
    untranslated
}

/// Drops the suffixes players add to a title: "Song - Live", "Song (feat. X)".
/// A title that is all decoration — "(Don't Fear) The Reaper" — keeps its own
/// text, because an empty one matches every song in the results.
fn plain_title(title: &str) -> String {
    let plain = title.split(" - ").next().unwrap_or(title);
    let plain = plain.split(['(', '（', '[']).next().unwrap_or(plain).trim();
    if plain.is_empty() {
        title.trim().to_string()
    } else {
        plain.to_string()
    }
}

/// The lead artist; a search does worse with the whole billing.
fn first_artist(artist: &str) -> String {
    let lead = artist
        .split([',', '&', '/', ';'])
        .next()
        .unwrap_or(artist)
        .trim();
    if lead.is_empty() {
        artist.trim().to_string()
    } else {
        lead.to_string()
    }
}

/// The QQ results worth asking for a lyric, best first. More than one, because
/// a result can turn out to carry no words and the next best deserves its turn.
fn qq_songs(
    client: &reqwest::blocking::Client,
    title: &str,
    artist: &str,
    duration: Option<f64>,
) -> Vec<serde_json::Value> {
    let plain_title = plain_title(title);
    let plain_artist = first_artist(artist);
    // `true` where only an exact title will do; see the last query.
    let mut queries = vec![(format!("{title} {artist}"), false)];
    push_query(&mut queries, format!("{plain_title} {plain_artist}"), false);
    // Apple Music hands over romanized names for part of its Chinese
    // catalogue — 赵雷 arrives as "Lei Zhao" — and an artist the search cannot
    // place drags the song itself out of the results, because every other
    // track on the album matches the query just as poorly. The title alone
    // finds it, and taking only an exact title keeps that from turning into a
    // different song with a similar name.
    push_query(&mut queries, plain_title.clone(), true);

    for (query, exact_title) in queries {
        let Ok(response) = client
            .get(format!(
                "{SEARCH_URL}?w={}&format=json&n={SEARCH_RESULTS}&p=1",
                encode(&query)
            ))
            .header("Referer", REFERER)
            .send()
        else {
            continue;
        };
        let Ok(body) = response.text() else { continue };
        let Ok(search) = serde_json::from_str::<serde_json::Value>(strip_jsonp(&body)) else {
            continue;
        };
        let Some(songs) = search["data"]["song"]["list"].as_array() else {
            continue;
        };
        let ranked = ranked_matches(songs, &plain_title, &plain_artist, duration, exact_title);
        if !ranked.is_empty() {
            return ranked.into_iter().take(ATTEMPTS).cloned().collect();
        }
    }
    Vec::new()
}

/// Queues a query unless an earlier one already says the same thing.
fn push_query(queries: &mut Vec<(String, bool)>, query: String, exact_title: bool) {
    let query = query.trim().to_string();
    if query.is_empty() || queries.iter().any(|(queued, _)| *queued == query) {
        return;
    }
    queries.push((query, exact_title));
}

fn fetch_qq_qrc_song(
    client: &reqwest::blocking::Client,
    song: &serde_json::Value,
) -> Option<FetchedLyrics> {
    let id = song["songid"].as_i64()?;
    let lines = fetch_qrc(client, id)?;
    Some(FetchedLyrics {
        provider: Provider::QqQrc,
        timed: has_timing(&lines),
        lines,
    })
}

fn fetch_qq_lrc_song(
    client: &reqwest::blocking::Client,
    song: &serde_json::Value,
) -> Option<FetchedLyrics> {
    let song_mid = song["songmid"].as_str()?.to_string();
    if song_mid.is_empty() {
        return None;
    }
    for attempt in 0..2 {
        if attempt > 0 {
            thread::sleep(Duration::from_millis(400));
        }
        let Some(payload) = lyric_payload(client, &song_mid) else {
            continue;
        };
        let lines = parse_lrc(payload["lyric"].as_str().unwrap_or_default());
        if !lines.is_empty() {
            return Some(FetchedLyrics {
                provider: Provider::QqLrc,
                timed: has_timing(&lines),
                lines: with_translation(
                    lines,
                    parse_lrc(payload["trans"].as_str().unwrap_or_default()),
                ),
            });
        }
    }
    None
}

fn fetch_netease(
    client: &reqwest::blocking::Client,
    title: &str,
    artist: &str,
    album: &str,
    duration: Option<f64>,
) -> Option<FetchedLyrics> {
    let query = format!("{} {}", plain_title(title), first_artist(artist));
    let search = client
        .get(format!(
            "https://music.163.com/api/search/get/web?s={}&type=1&offset=0&total=true&limit=10",
            encode(&query)
        ))
        .header("Referer", "https://music.163.com/")
        .send()
        .ok()?
        .text()
        .ok()?;
    let search: serde_json::Value = serde_json::from_str(&search).ok()?;
    let songs = search["result"]["songs"].as_array()?;
    let song = best_netease_match(songs, title, artist, album, duration)?;
    let id = song["id"].as_i64()?;
    let body = client
        .get(format!(
            "https://music.163.com/api/song/lyric?id={id}&lv=1&kv=1&tv=-1"
        ))
        .header("Referer", "https://music.163.com/")
        .send()
        .ok()?
        .text()
        .ok()?;
    let payload: serde_json::Value = serde_json::from_str(&body).ok()?;

    let lines = if let Some(yrc) = payload["yrc"]["lyric"].as_str() {
        let lines = parse_yrc(yrc);
        if lines.is_empty() {
            parse_lrc(payload["lrc"]["lyric"].as_str().unwrap_or_default())
        } else {
            lines
        }
    } else {
        parse_lrc(payload["lrc"]["lyric"].as_str().unwrap_or_default())
    };
    if lines.is_empty() {
        return None;
    }
    Some(FetchedLyrics {
        provider: Provider::NetEase,
        timed: has_timing(&lines),
        lines: with_translation(
            lines,
            parse_lrc(payload["tlyric"]["lyric"].as_str().unwrap_or_default()),
        ),
    })
}

fn best_netease_match<'a>(
    songs: &'a [serde_json::Value],
    title: &str,
    artist: &str,
    album: &str,
    duration: Option<f64>,
) -> Option<&'a serde_json::Value> {
    let wanted_title = normalize(&plain_title(title));
    let wanted_artist = normalize(&first_artist(artist));
    let wanted_album = normalize(album);
    songs
        .iter()
        .filter(|song| {
            let name = normalize(song["name"].as_str().unwrap_or_default());
            name == wanted_title || name.contains(&wanted_title) || wanted_title.contains(&name)
        })
        .max_by_key(|song| {
            let name = normalize(song["name"].as_str().unwrap_or_default());
            let artists = song["artists"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item["name"].as_str())
                        .map(normalize)
                        .collect::<Vec<_>>()
                        .join("/")
                })
                .unwrap_or_default();
            let album_name = normalize(song["album"]["name"].as_str().unwrap_or_default());
            let mut score = if name == wanted_title {
                4
            } else if name.contains(&wanted_title) || wanted_title.contains(&name) {
                2
            } else {
                0
            };
            if !wanted_artist.is_empty()
                && (artists.contains(&wanted_artist) || wanted_artist.contains(&artists))
            {
                score += 3;
            }
            if !wanted_album.is_empty()
                && (album_name == wanted_album
                    || album_name.contains(&wanted_album)
                    || wanted_album.contains(&album_name))
            {
                score += 1;
            }
            if let (Some(wanted), Some(actual)) =
                (duration, song["duration"].as_f64().map(|ms| ms / 1000.0))
            {
                if (wanted - actual).abs() < 3.0 {
                    score += 1;
                }
            }
            score
        })
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
    crate::platform::to_simplified(value)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

/// The results that could be this song, best first: an exact title by the same
/// artist, running the length the player reports. The search endpoint
/// otherwise happily puts covers and remixes above the recording itself.
fn ranked_matches<'a>(
    songs: &'a [serde_json::Value],
    title: &str,
    artist: &str,
    duration: Option<f64>,
    exact_title: bool,
) -> Vec<&'a serde_json::Value> {
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

    let mut matches: Vec<(&serde_json::Value, i32, f64, usize)> = Vec::new();
    for (rank, song) in songs.iter().enumerate() {
        let (title_score, artist_score) = score(song);
        // A recording of a different length is a different recording. A cover
        // carries the same words on a timeline of its own, which reads as
        // lyrics that drift — worse than no lyrics at all.
        let gap = gap(song);
        if gap.is_some_and(|gap| gap > DURATION_SLACK) {
            continue;
        }
        let length_score = match gap {
            Some(gap) if gap <= SAME_RECORDING => 2,
            Some(_) => 1,
            None => 0,
        };
        // Catalogues translate: Apple Music calls 西湖 "West Lake" and 痛仰乐队
        // "Miserable Faith", and neither word survives to be compared. The
        // search understood the query anyway — it answered with the song — so
        // a result it ranked at the top, running the length that is playing,
        // is taken on the engine's word. A title-only query gets no such
        // benefit: there the title is all that was asked.
        let vouched = !exact_title && rank < TRUSTED_RANK && length_score == 2;
        if (title_score == 0 && !vouched) || (exact_title && title_score < 2) {
            continue;
        }
        matches.push((
            song,
            title_score + artist_score + length_score,
            gap.unwrap_or(f64::MAX),
            rank,
        ));
    }
    // Score first, then the length closest to what is playing; where neither
    // separates two results, the order the endpoint returned them in stands.
    matches.sort_by(|left, right| {
        right.1.cmp(&left.1).then(left.2.total_cmp(&right.2)).then(left.3.cmp(&right.3))
    });
    matches.into_iter().map(|(song, ..)| song).collect()
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
        .map(|text| parse_translation(&text))
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
        let Some(rest) = row.strip_prefix('[') else {
            continue;
        };
        let Some((head, body)) = rest.split_once(']') else {
            continue;
        };
        let Some((start, _)) = head.split_once(',') else {
            continue;
        };
        let Ok(start): Result<f64, _> = start.trim().parse() else {
            continue;
        };

        let words = parse_qrc_words(body);
        let text: String = words.iter().map(|word| word.text.as_str()).collect();
        if text.trim().is_empty() {
            continue;
        }
        lines.push(LyricLine {
            at: start / 1000.0,
            text,
            translation: None,
            words,
        });
    }
    lines.sort_by(|a, b| a.at.total_cmp(&b.at));
    lines
}

/// NetEase's YRC uses the same timing data as QRC, without the XML wrapper or
/// encryption; both before-character and after-character layouts are accepted.
fn parse_yrc(raw: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    for row in raw.lines() {
        let Some(rest) = row.strip_prefix('[') else {
            continue;
        };
        let Some((head, body)) = rest.split_once(']') else {
            continue;
        };
        let Some((start, _)) = head.split_once(',') else {
            continue;
        };
        let Ok(start): Result<f64, _> = start.trim().parse() else {
            continue;
        };
        let words = parse_yrc_words(body);
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

/// YRC commonly puts each timing group before its character, while QRC puts
/// it after the character. Accept both forms because NetEase has served both
/// layouts over time.
fn parse_yrc_words(body: &str) -> Vec<LyricWord> {
    if !body.trim_start().starts_with('(') {
        return parse_qrc_words(body);
    }
    let mut words = Vec::new();
    let mut chars = body.chars().peekable();
    while chars.peek().is_some() {
        if chars.next() != Some('(') {
            continue;
        }
        let timing: String = chars.by_ref().take_while(|next| *next != ')').collect();
        let mut values = timing.split(',');
        let Some(at) = values
            .next()
            .and_then(|value| value.trim().parse::<f64>().ok())
        else {
            continue;
        };
        let Some(duration) = values
            .next()
            .and_then(|value| value.trim().parse::<f64>().ok())
        else {
            continue;
        };
        let mut text = String::new();
        while let Some(next) = chars.peek().copied() {
            if next == '(' {
                break;
            }
            text.push(next);
            chars.next();
        }
        if !text.is_empty() {
            words.push(LyricWord {
                at: at / 1000.0,
                duration: duration / 1000.0,
                text,
            });
        }
    }
    words
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
        let mut values = timing.split(',');
        let Some(at) = values
            .next()
            .and_then(|value| value.trim().parse::<f64>().ok())
        else {
            text.push('(');
            text.push_str(&timing);
            text.push(')');
            continue;
        };
        let Some(duration) = values
            .next()
            .and_then(|value| value.trim().parse::<f64>().ok())
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

/// Parses a translation payload while preserving literal parentheses. QRC
/// timing groups are removed only when they contain numeric timing values.
fn parse_translation(document: &str) -> Vec<LyricLine> {
    let content = qrc_content(document).unwrap_or(document);
    let mut lines = Vec::new();
    for row in content.lines() {
        let Some(rest) = row.strip_prefix('[') else {
            continue;
        };
        let Some((head, body)) = rest.split_once(']') else {
            continue;
        };
        let Some((start, _)) = head.split_once(',') else {
            continue;
        };
        let Ok(start): Result<f64, _> = start.trim().parse() else {
            continue;
        };
        let words = parse_qrc_words(body);
        let text = if words.is_empty() {
            body.trim().to_string()
        } else {
            words.into_iter().map(|word| word.text).collect()
        };
        if !text.trim().is_empty() {
            lines.push(LyricLine {
                at: start / 1000.0,
                text,
                translation: None,
                words: Vec::new(),
            });
        }
    }
    if lines.is_empty() {
        parse_lrc(content)
    } else {
        lines.sort_by(|a, b| a.at.total_cmp(&b.at));
        lines
    }
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
            lines.push(LyricLine {
                at,
                text: text.to_string(),
                translation: None,
                words: Vec::new(),
            });
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
            .map(|translation| translation.text.trim().to_string())
            .filter(|text| !text.is_empty() && *text != line.text.trim());
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
        ranked_matches(songs, title, artist, None, exact)
            .first()
            .map(|song| song["songname"].as_str().unwrap())
    }

    fn pick_lasting<'a>(
        songs: &'a [serde_json::Value],
        title: &str,
        artist: &str,
        duration: f64,
    ) -> Option<&'a str> {
        ranked_matches(songs, title, artist, Some(duration), false)
            .first()
            .map(|song| song["songname"].as_str().unwrap())
    }

    #[test]
    fn keeps_the_best_ranked_of_equally_good_matches() {
        // A duet scores exactly like the original, and the endpoint put the
        // original first.
        let songs = [song("无法长大", &["赵雷"]), song("无法长大", &["张大为", "赵雷"])];
        let ranked = ranked_matches(&songs, "无法长大", "赵雷", None, false);
        assert!(std::ptr::eq(ranked[0], &songs[0]));
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
        let ranked = ranked_matches(&songs, "小宇", "A-Yue Chang", Some(228.0), false);
        assert!(std::ptr::eq(ranked[0], &songs[1]));
    }

    #[test]
    fn takes_a_top_result_of_the_right_length_on_the_search_s_word() {
        // West Lake by Miserable Faith is 西湖 by 痛仰乐队; not a word matches.
        let songs = [song_of("西湖", &["痛仰乐队"], 253), song_of("West Lake", &["K3CIM"], 125)];
        assert_eq!(pick_lasting(&songs, "West Lake", "Miserable Faith", 251.0), Some("西湖"));
    }

    #[test]
    fn keeps_a_result_whose_length_is_unknown() {
        let songs = [song("小宇", &["张震岳"])];
        assert_eq!(pick_lasting(&songs, "小宇", "A-Yue Chang", 228.0), Some("小宇"));
    }

    #[test]
    fn hears_nothing_in_a_placeholder() {
        let line = |text: &str| LyricLine {
            at: 0.0,
            text: text.to_string(),
            translation: None,
            words: Vec::new(),
        };
        assert!(!has_words(&[line("此歌曲为没有填词的纯音乐，请您欣赏")]));
        assert!(!has_words(&[line("作词：K3CIM"), line("编曲：K3CIM")]));
        assert!(has_words(&[line("作词：林夕"), line("当时桌上有一杯茶")]));
    }

    #[test]
    fn asks_for_a_translation_only_where_one_would_say_something() {
        let line = |text: &str, translation: Option<&str>| LyricLine {
            at: 0.0,
            text: text.to_string(),
            translation: translation.map(str::to_string),
            words: Vec::new(),
        };
        // A Chinese lyric is the one the panel reads; nothing to add.
        assert!(!wants_translation(&[line("当时桌上有一杯茶", None)]));
        assert!(wants_translation(&[line("처음 본 널 기억해", None)]));
        assert!(wants_translation(&[line("We are, we are", None)]));
        assert!(!has_translation(&[line("Darling", Some("  "))]));
        assert!(has_translation(&[line("Darling", Some("亲爱的"))]));
    }

    #[test]
    fn queues_each_query_once() {
        let mut queries = vec![("Song Artist".to_string(), false)];
        push_query(&mut queries, "Song Artist".to_string(), false);
        push_query(&mut queries, "Song ".to_string(), true);
        push_query(&mut queries, "Song".to_string(), true);
        assert_eq!(queries, [("Song Artist".to_string(), false), ("Song".to_string(), true)]);
    }

    fn media(source_id: &str, app_name: &str) -> MediaState {
        MediaState {
            title: "Song".into(),
            artist: "Artist".into(),
            album: "Album".into(),
            app_name: app_name.into(),
            source_id: source_id.into(),
            playing: true,
            duration: Some(180.0),
            elapsed: Some(10.0),
            elapsed_at: 0.0,
            artwork: None,
        }
    }

    #[test]
    fn provider_order_follows_player() {
        assert_eq!(
            provider_order(PlayerKind::QqMusic),
            &[Provider::QqQrc, Provider::QqLrc, Provider::NetEase]
        );
        assert_eq!(
            provider_order(PlayerKind::NetEase),
            &[Provider::NetEase, Provider::QqQrc, Provider::QqLrc]
        );
        assert_eq!(
            provider_order(PlayerKind::Spotify),
            &[Provider::QqQrc, Provider::NetEase, Provider::QqLrc]
        );
        assert_eq!(
            provider_order(PlayerKind::Other),
            provider_order(PlayerKind::Spotify)
        );
    }

    #[test]
    fn detects_known_player_identifiers() {
        assert_eq!(
            player_kind(&media("com.tencent.QQMusicMac", "QQ 音乐")),
            PlayerKind::QqMusic
        );
        assert_eq!(
            player_kind(&media("com.netease.163music", "网易云音乐")),
            PlayerKind::NetEase
        );
        assert_eq!(
            player_kind(&media("com.spotify.client", "Spotify")),
            PlayerKind::Spotify
        );
    }

    #[test]
    fn parses_lrc_and_translation() {
        let lines = parse_lrc("[00:01.50]first\n[00:03.00]second");
        let translated = with_translation(lines, parse_lrc("[00:01.50]第一句"));
        assert_eq!(translated.len(), 2);
        assert!((translated[0].at - 1.5).abs() < f64::EPSILON);
        assert_eq!(translated[0].translation.as_deref(), Some("第一句"));
    }

    #[test]
    fn parses_yrc_words() {
        let lines = parse_yrc("[1000,1000]he(1000,400)llo(1400,600)");
        assert_eq!(lines[0].text, "hello");
        assert_eq!(lines[0].words.len(), 2);
        assert!((lines[0].words[1].at - 1.4).abs() < f64::EPSILON);

        let prefixed = parse_yrc("[1000,1000](1000,400,0)he(1400,600,0)llo");
        assert_eq!(prefixed[0].text, "hello");
        assert_eq!(prefixed[0].words.len(), 2);
        assert!((prefixed[0].words[0].at - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parses_qrc_content_and_character_timings() {
        let lines = parse_qrc(r#"<LyricContent="[0,1000]he(0,500)llo(500,500)"/>"#);
        assert_eq!(lines[0].text, "hello");
        assert_eq!(lines[0].words.len(), 2);
        assert!((lines[0].words[1].duration - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn accepts_timed_results_without_translation() {
        let mut lines = parse_lrc("[00:01.00]hello\n[00:02.00]world");
        assert!(acceptable(&FetchedLyrics {
            provider: Provider::QqLrc,
            timed: has_timing(&lines),
            lines: lines.clone(),
        }));
        lines[0].translation = Some("你好".into());
        assert!(acceptable(&FetchedLyrics {
            provider: Provider::QqLrc,
            timed: true,
            lines
        }));
    }

    #[test]
    fn translation_pairing_ignores_mismatched_timestamps() {
        let lines = with_translation(parse_lrc("[00:01.00]hello"), parse_lrc("[00:03.00]你好"));
        assert!(lines[0].translation.is_none());
        assert!(acceptable(&FetchedLyrics {
            provider: Provider::QqLrc,
            timed: true,
            lines
        }));
    }

    #[test]
    fn qrc_translation_preserves_literal_parentheses() {
        let lines = parse_qrc(r#"<LyricContent="[1000,1000]Hello(1200,800)"/>"#);
        let translations =
            parse_translation(r#"<LyricContent="[1000,1000]你好（朋友）(1000,1000)"/>"#);
        let lines = with_translation(lines, translations);
        assert_eq!(lines[0].translation.as_deref(), Some("你好（朋友）"));
    }

    #[test]
    fn netease_match_uses_millisecond_duration_and_filters_titles_first() {
        let songs = vec![
            serde_json::json!({ "name": "Wrong", "duration": 251253, "artists": [{"name":"Artist"}], "album": {"name":"Album"} }),
            serde_json::json!({ "name": "Song", "duration": 251253, "artists": [{"name":"Artist"}], "album": {"name":"Album"} }),
        ];
        let matched = best_netease_match(&songs, "Song", "Artist", "Album", Some(251.0)).unwrap();
        assert_eq!(matched["name"].as_str(), Some("Song"));
    }
}
