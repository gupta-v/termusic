use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use id3::TagLike;
use id3::Version::Id3v24;
use regex::Regex;
use serde_json::Value;
use shell_words;
use termusiclib::invidious::YoutubeVideo;
use termusiclib::track::DurationFmtShort;
use termusiclib::utils::get_parent_folder;
use tuirealm::props::{
    AttrValue, Attribute, HorizontalAlignment, LineStatic, Style, TableBuilder, Title,
};
use ytd_rs::{Arg, YoutubeDL};

use super::Model;
use crate::ui::ids::Id;
use crate::ui::msg::{Msg, YSMsg};

/// How many results are shown per page in the search popup.
const RESULTS_PER_PAGE: usize = 5;
/// How many results to fetch from `yt-dlp` per search (a few pages' worth).
const RESULTS_TO_FETCH: usize = 15;

// static RE_FILENAME_YTDLP: LazyLock<Regex> =
//     LazyLock::new(|| Regex::new(r"\[ExtractAudio\] Destination: (?P<name>.*)\.mp3").unwrap());

static RE_FILENAME_YTDLP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[ExtractAudio\] Destination: (?P<name>.*?\.opus)").unwrap());

static RE_FILENAME_YTDLP_DUPLICATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[ExtractAudio\] Not converting audio (?P<name>.*?\.opus)").unwrap()
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YoutubeData {
    /// All fetched results (not just the currently-shown page).
    pub items: Vec<YoutubeVideo>,
    pub page: u32,
}

impl Default for YoutubeData {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            page: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct YoutubeOptions {
    pub data: YoutubeData,
}

impl YoutubeOptions {
    /// Get an item by its index within the currently-displayed page (0-based).
    pub fn get_by_index(&self, page_relative_index: usize) -> Result<&YoutubeVideo> {
        let absolute_index = self.page_start() + page_relative_index;
        self.data
            .items
            .get(absolute_index)
            .ok_or_else(|| anyhow!("index not found"))
    }

    /// Move to the previous page, if any. All results are already fetched, so this is instant.
    pub fn go_prev_page(&mut self) -> bool {
        if self.data.page > 1 {
            self.data.page -= 1;
            true
        } else {
            false
        }
    }

    /// Move to the next page, if any. All results are already fetched, so this is instant.
    pub fn go_next_page(&mut self) -> bool {
        if self.data.page < self.last_page() {
            self.data.page += 1;
            true
        } else {
            false
        }
    }

    fn page_start(&self) -> usize {
        (self.data.page as usize - 1) * RESULTS_PER_PAGE
    }

    fn last_page(&self) -> u32 {
        if self.data.items.is_empty() {
            1
        } else {
            u32::try_from((self.data.items.len() - 1) / RESULTS_PER_PAGE + 1).unwrap_or(1)
        }
    }

    #[must_use]
    pub const fn page(&self) -> u32 {
        self.data.page
    }

    pub fn is_empty(&self) -> bool {
        self.data.items.is_empty()
    }

    /// The items visible on the currently-displayed page.
    pub fn items(&self) -> &[YoutubeVideo] {
        let start = self.page_start();
        let end = (start + RESULTS_PER_PAGE).min(self.data.items.len());
        self.data.items.get(start..end).unwrap_or(&[])
    }
}

impl Model {
    pub fn youtube_options_download(&mut self, index: usize, current_node: &Path) -> Result<()> {
        // download from search result here
        if let Ok(item) = self.youtube_options.get_by_index(index) {
            let url = format!("https://www.youtube.com/watch?v={}", item.video_id);
            let title = item.title.clone();
            self.youtube_dl(url.as_ref(), &title, current_node)
                .context("YTDL Download")?;
        }
        Ok(())
    }

    /// This function requires to be run in a tokio Runtime context
    ///
    /// Uses `yt-dlp`'s own search extractor (`ytsearchN:query`) instead of the public
    /// Invidious mirrors, which are frequently bot-walled / rate-limited / offline.
    pub fn youtube_options_search(&mut self, keyword: String) {
        let tx = self.tx_to_main.clone();
        tokio::spawn(async move {
            match ytdlp_search(&keyword, RESULTS_TO_FETCH).await {
                Ok(items) => {
                    let youtube_options = YoutubeOptions {
                        data: YoutubeData { items, page: 1 },
                    };
                    tx.send(Msg::YoutubeSearch(YSMsg::YoutubeSearchSuccess(
                        youtube_options,
                    )))
                    .ok();
                }
                Err(e) => {
                    tx.send(Msg::YoutubeSearch(YSMsg::YoutubeSearchFail(e.to_string())))
                        .ok();
                }
            }
        });
    }

    /// Move to the previous page of already-fetched results. Instant, no network needed.
    pub fn youtube_options_prev_page(&mut self) {
        if self.youtube_options.go_prev_page() {
            self.sync_youtube_options();
        }
    }

    /// Move to the next page of already-fetched results. Instant, no network needed.
    pub fn youtube_options_next_page(&mut self) {
        if self.youtube_options.go_next_page() {
            self.sync_youtube_options();
        }
    }

    pub fn sync_youtube_options(&mut self) {
        if self.youtube_options.is_empty() {
            let table = TableBuilder::default()
                .add_col(LineStatic::from("No results."))
                .add_col(LineStatic::from(
                    "Nothing was found in 10 seconds, connection issue encountered.",
                ))
                .build();
            self.app
                .attr(
                    &Id::YoutubeSearchTablePopup,
                    Attribute::Content,
                    AttrValue::Table(table),
                )
                .ok();
            return;
        }

        let mut table: TableBuilder = TableBuilder::default();
        for (idx, record) in self.youtube_options.items().iter().enumerate() {
            if idx > 0 {
                table.add_row();
            }
            let duration = DurationFmtShort(Duration::from_secs(record.length_seconds));
            let duration_string = format!("[{duration:^10.10}]");

            table
                .add_col(LineStatic::from(duration_string))
                .add_col(LineStatic::styled(
                    record.title.clone(),
                    Style::new().bold(),
                ));
        }
        let table = table.build();
        self.app
            .attr(
                &Id::YoutubeSearchTablePopup,
                Attribute::Content,
                AttrValue::Table(table),
            )
            .ok();

        let title = format!(
            "\u{2500}\u{2500}\u{2500} Page {} \u{2500}\u{2500}\u{2500}\u{2524} {} \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            self.youtube_options.page(),
            "Tab/Shift+Tab switch pages",
        );
        self.app
            .attr(
                &Id::YoutubeSearchTablePopup,
                Attribute::Title,
                AttrValue::Title(Title::from(title).alignment(HorizontalAlignment::Left)),
            )
            .ok();
    }

    #[allow(clippy::too_many_lines)]
    pub fn youtube_dl(&mut self, url: &str, title: &str, current_node: &Path) -> Result<()> {
        let path: PathBuf = get_parent_folder(current_node).into_owned();
        let config_tui = self.config_tui.read();
        let mut args = vec![
            Arg::new("--no-playlist"),
            Arg::new("--extract-audio"),
            Arg::new_with_arg("--audio-format", "opus"),
            Arg::new("--add-metadata"),
            // NOT --embed-thumbnail: embedding a picture into the Opus/Ogg container corrupts
            // its Ogg page framing for `lofty` (the metadata reader termusic uses) - it throws
            // `OggPage(MissingMagic)` and title/artist/duration/lyrics all silently read as
            // empty, even though ffmpeg/ffprobe still play the file fine. `--write-thumbnail`
            // alone (a separate .webp next to the track) avoids this entirely.
            Arg::new("--write-thumbnail"),
            // Fallback artist = channel/uploader name, applied first so the title-split rule
            // below can override it when the video's title actually contains "Artist - Title"
            // (many videos, especially non-music-panel ones, have no artist tag otherwise).
            Arg::new_with_arg("--parse-metadata", "%(uploader)s:%(meta_artist)s"),
            Arg::new_with_arg("--metadata-from-title", "%(artist)s - %(title)s"),
            #[cfg(target_os = "windows")]
            Arg::new("--restrict-filenames"),
            Arg::new("--write-sub"),
            Arg::new("--all-subs"),
            Arg::new_with_arg("--convert-subs", "lrc"),
            Arg::new_with_arg("--output", "%(title).90s.%(ext)s"),
            // Note: deliberately NOT ".thumbnails" - that name is already used by termusic's
            // own internal album-art cache directory (has a `.database_uuid` marker file and
            // numeric-ID-keyed images); reusing it would mix arbitrary download filenames into
            // that system-managed cache.
            Arg::new_with_arg("--output", "thumbnail:.yt-thumbnails/%(title).90s.%(ext)s"),
        ];
        let extra_args = parse_args(&config_tui.settings.ytdlp.extra_args)
            .context("Parsing config `extra_ytdlp_args`")?;
        let mut extra_args_parsed = convert_to_args(extra_args);
        if !extra_args_parsed.is_empty() {
            args.append(&mut extra_args_parsed);
        }

        let ytd = YoutubeDL::new(&path, args, url)?;
        let tx = self.tx_to_main.clone();

        // avoid full string clones when sending via a channel
        let url: Arc<str> = Arc::from(url);
        let title = title.to_string();

        thread::spawn(move || -> Result<()> {
            tx.send(Msg::YoutubeSearch(YSMsg::Download(YTDLMsg::Start(
                url.clone(),
                title.clone(),
            ))))
            .ok();
            // start download
            let download = ytd.download();

            // check what the result is and print out the path to the download or the error
            match download {
                Ok(result) => {
                    // left for debug
                    // eprintln!("{}", result.output());
                    tx.send(Msg::YoutubeSearch(YSMsg::Download(YTDLMsg::Success(
                        url.clone(),
                        title.clone(),
                    ))))
                    .ok();
                    // here we extract the full file name from download output
                    if let Some(file_fullname) =
                        extract_filepath(result.output(), &path.to_string_lossy())
                    {
                        tx.send(Msg::YoutubeSearch(YSMsg::Download(YTDLMsg::Completed(
                            url,
                            Some(file_fullname.clone()),
                        ))))
                        .ok();

                        // here we remove downloaded live_chat.json file
                        remove_downloaded_json(&path, &file_fullname);

                        embed_downloaded_lrc(&path, &file_fullname);
                    } else {
                        tx.send(Msg::YoutubeSearch(YSMsg::Download(YTDLMsg::Completed(
                            url, None,
                        ))))
                        .ok();
                    }
                }
                Err(e) => {
                    tx.send(Msg::YoutubeSearch(YSMsg::Download(YTDLMsg::Err(
                        url.clone(),
                        "youtube music".to_string(),
                        e.to_string(),
                    ))))
                    .ok();
                    tx.send(Msg::YoutubeSearch(YSMsg::Download(YTDLMsg::Completed(
                        url, None,
                    ))))
                    .ok();
                }
            }
            Ok(())
        });
        Ok(())
    }
}

/// Search `YouTube` via `yt-dlp`'s own search extractor (`ytsearchN:query`), bypassing the
/// public Invidious mirrors entirely (they are frequently bot-walled, rate-limited, or
/// simply offline - see the fallback list in `termusiclib::invidious`).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
async fn ytdlp_search(query: &str, count: usize) -> Result<Vec<YoutubeVideo>> {
    let search_spec = format!("ytsearch{count}:{query}");

    let output = tokio::process::Command::new("yt-dlp")
        .args([
            "--flat-playlist",
            "--dump-json",
            "--no-warnings",
            "--skip-download",
            &search_spec,
        ])
        .output()
        .await
        .context("Failed to run yt-dlp (is it installed and on PATH?)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("yt-dlp search failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let results: Vec<YoutubeVideo> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|entry| {
            let video_id = entry.get("id")?.as_str()?.to_string();
            let title = entry
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Unknown title")
                .to_string();
            let length_seconds = entry
                .get("duration")
                .and_then(Value::as_f64)
                .map_or(0, |d| d as u64);

            Some(YoutubeVideo {
                title,
                length_seconds,
                video_id,
            })
        })
        .collect();

    if results.is_empty() {
        bail!("No results found (or yt-dlp's output could not be parsed)");
    }

    Ok(results)
}

pub type YTDLMsgURL = Arc<str>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum YTDLMsg {
    /// Indicates a Start of a download.
    ///
    /// `(Url, Title)`
    Start(YTDLMsgURL, String),
    /// Indicates the Download was a Success, though termusic post-processing is not done yet.
    ///
    /// `(Url, Title)`
    Success(YTDLMsgURL, String),
    /// Indicates the Download thread finished in both Success or Error.
    ///
    /// `(Url, Filename)`
    Completed(YTDLMsgURL, Option<String>),
    /// Indicates that the Download has Errored and has been aborted.
    ///
    /// `(Url, Title, ErrorAsString)`
    Err(YTDLMsgURL, String, String),
}

// This just parsing the output from youtubedl to get the audio path
// This is used because we need to get the song name
// example ~/path/to/song/song.mp3
//
fn extract_filepath(output: &str, dir: &str) -> Option<String> {
    if let Some(cap) = RE_FILENAME_YTDLP.captures(output)
        && let Some(c) = cap.name("name")
    {
        let filename = format!("{dir}/{}", c.as_str());
        return Some(filename);
    }

    if let Some(cap) = RE_FILENAME_YTDLP_DUPLICATE.captures(output)
        && let Some(c) = cap.name("name")
    {
        let filename = format!("{dir}/{}", c.as_str());
        return Some(filename);
    }

    None
}

fn remove_downloaded_json(path: &Path, file_fullname: &str) {
    let files = walkdir::WalkDir::new(path).follow_links(true);
    for f in files
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|f| {
            let p = Path::new(f.file_name());
            p.extension().is_some_and(|ext| ext == "json")
        })
        .filter(|f| {
            let path_json = Path::new(f.file_name());
            let p1: &Path = Path::new(file_fullname);
            path_json.file_stem().is_some_and(|stem_lrc| {
                p1.file_stem().is_some_and(|p_base| {
                    stem_lrc
                        .to_string_lossy()
                        .contains(p_base.to_string_lossy().as_ref())
                })
            })
        })
    {
        std::fs::remove_file(f.path()).ok();
    }
}

fn embed_downloaded_lrc(path: &Path, file_fullname: &str) {
    // ID3v2 is an MP3-specific tagging format; writing it into an Opus/Ogg file corrupts the
    // Ogg container's page framing (lofty then fails to read ANY metadata at all - title,
    // artist, duration, everything). Since yt-dlp downloads are always opus, just leave any
    // `.lrc` files as plain sidecar files instead of embedding (and corrupting).
    if Path::new(file_fullname)
        .extension()
        .and_then(|e| e.to_str())
        != Some("mp3")
    {
        return;
    }

    let mut id3_tag = if let Ok(tag) = id3::Tag::read_from_path(file_fullname) {
        tag
    } else {
        let mut tags = id3::Tag::new();
        let file_path = Path::new(file_fullname);
        if let Some(p_base) = file_path.file_stem() {
            tags.set_title(p_base.to_string_lossy());
        }
        tags.write_to_path(file_path, Id3v24).ok();
        tags
    };

    // here we add all downloaded lrc file
    let files = walkdir::WalkDir::new(path).follow_links(true);

    for entry in files
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|f| f.file_type().is_file())
        .filter(|f| {
            let name = f.file_name();
            let p = Path::new(&name);
            p.extension().is_some_and(|ext| ext == "lrc")
        })
        .filter(|f| {
            let path_lrc = Path::new(f.file_name());
            let p1: &Path = Path::new(file_fullname);
            path_lrc.file_stem().is_some_and(|stem_lrc| {
                p1.file_stem().is_some_and(|p_base| {
                    stem_lrc
                        .to_string_lossy()
                        .contains(p_base.to_string_lossy().as_ref())
                })
            })
        })
    {
        let path_lrc = Path::new(entry.file_name());
        let mut lang_ext = "eng".to_string();
        if let Some(p_short) = path_lrc.file_stem() {
            let p2 = Path::new(p_short);
            if let Some(ext2) = p2.extension() {
                lang_ext = ext2.to_string_lossy().to_string();
            }
        }
        let lyric_string = std::fs::read_to_string(entry.path());
        id3_tag.add_frame(id3::frame::Lyrics {
            lang: "eng".to_string(),
            description: lang_ext,
            text: lyric_string.unwrap_or_else(|_| String::from("[00:00:01] No lyric")),
        });
        std::fs::remove_file(entry.path()).ok();
    }

    id3_tag.write_to_path(file_fullname, Id3v24).ok();
}

#[derive(Debug, Clone, PartialEq)]
enum ArgOrVal {
    ArgumentWithVal(String),
    Flag(String),
    Argument(String),
    Positional(String),
}

/// Parse the input shell-like string into a Vector of `argument` and `maybe argument value`.
fn parse_args(input: &str) -> Result<Vec<ArgOrVal>, shell_words::ParseError> {
    let result = shell_words::split(input)?
        .into_iter()
        .map(|token| {
            if token.starts_with("--") {
                if token.contains('=') {
                    ArgOrVal::ArgumentWithVal(token)
                } else {
                    ArgOrVal::Argument(token)
                }
            } else if token.starts_with('-') {
                ArgOrVal::Flag(token)
            } else {
                ArgOrVal::Positional(token)
            }
        })
        .collect();
    Ok(result)
}

/// Convert the `argument, maybe value` vector to [ytdrs Arguments](Arg).
fn convert_to_args(extra_args: Vec<ArgOrVal>) -> Vec<Arg> {
    // This capacity *may* be a little inaccurate, but should broadly reflect what we need
    let mut extra_args_parsed = Vec::with_capacity(extra_args.len());

    // store the last "maybe incomplete" argument here
    // this has to be done because ytdrs `Arg` are non-modifiable after creation.
    let mut last_arg: Option<String> = None;

    for val in extra_args {
        // push last arg to the array, before processing a new one
        match &val {
            ArgOrVal::ArgumentWithVal(_) | ArgOrVal::Argument(_) | ArgOrVal::Flag(_) => {
                if let Some(v) = last_arg.take() {
                    extra_args_parsed.push(Arg::new(&v));
                }
            }
            ArgOrVal::Positional(_) => (),
        }

        match val {
            ArgOrVal::ArgumentWithVal(v) | ArgOrVal::Flag(v) => {
                extra_args_parsed.push(Arg::new(&v));
            }
            ArgOrVal::Argument(v) => {
                last_arg = Some(v);
            }
            ArgOrVal::Positional(v) => {
                let Some(last_arg) = last_arg.take() else {
                    // in case there is a positional but no previous argument to combine with, skip the positional with a error
                    // maybe we should error instead?
                    error!("Positional without previous argument! {v:#?}");
                    continue;
                };
                extra_args_parsed.push(Arg::new_with_arg(&last_arg, &v));
            }
        }
    }

    if let Some(remainder) = last_arg {
        extra_args_parsed.push(Arg::new(&remainder));
    }

    extra_args_parsed
}

#[cfg(test)]
mod tests {

    use crate::ui::model::youtube_options::extract_filepath;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_youtube_output_parsing() {
        // #[cfg(not(feature = "yt-dlp"))]
        // assert_eq!(
        //     extract_filepath(
        //         r"sdflsdf [ffmpeg] Destination: 观众说“小哥哥，到饭点了”《干饭人之歌》走，端起饭盆干饭去.mp3 sldflsdfj",
        //         "/tmp"
        //     )
        //     .unwrap(),
        //     "/tmp/观众说“小哥哥，到饭点了”《干饭人之歌》走，端起饭盆干饭去.mp3".to_string()
        // );
        assert_eq!(
            extract_filepath(
                r"sdflsdf [ExtractAudio] Destination: 观众说“小哥哥，到饭点了”《干饭人之歌》走，端起饭盆干饭去.opus sldflsdfj",
                "/tmp"
            )
            .unwrap(),
            "/tmp/观众说“小哥哥，到饭点了”《干饭人之歌》走，端起饭盆干饭去.opus".to_string()
        );
    }
}
