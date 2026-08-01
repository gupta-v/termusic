//! SPDX-License-Identifier: MIT

#[cfg(any(
    feature = "cover-viuer-iterm",
    feature = "cover-viuer-kitty",
    feature = "cover-viuer-sixel"
))]
use std::io::Write;

#[cfg(any(
    feature = "cover-viuer-iterm",
    feature = "cover-viuer-kitty",
    feature = "cover-viuer-sixel"
))]
use anyhow::Context;
use anyhow::Result;
use image::DynamicImage;
use std::path::{Path, PathBuf};
use termusiclib::track::MediaTypes;
use tokio::runtime::Handle;
use tuirealm::ratatui::layout::Rect;

use crate::ui::ids::{Id, IdConfigEditor, IdTagEditor};
use crate::ui::model::{Model, TxToMain, ViuerSupported};
use crate::ui::msg::{CoverDLResult, ImageWrapper, Msg, XYWHMsg};

/// Look for a `.yt-thumbnails/<same-stem>.<ext>` file next to `track_path`, matching the
/// output template used for yt-dlp downloads (see `youtube_options.rs`).
fn sidecar_thumbnail_path(track_path: &Path) -> Option<PathBuf> {
    let parent = track_path.parent()?;
    let stem = track_path.file_stem()?.to_string_lossy();
    let thumb_dir = parent.join(".yt-thumbnails");

    ["webp", "jpg", "jpeg", "png"]
        .iter()
        .map(|ext| thumb_dir.join(format!("{stem}.{ext}")))
        .find(|candidate| candidate.is_file())
}

impl Model {
    pub fn xywh_move_left(&mut self) {
        self.xywh.move_left();
        self.update_photo().ok();
    }

    pub fn xywh_move_right(&mut self) {
        self.xywh.move_right();
        self.update_photo().ok();
    }

    pub fn xywh_move_up(&mut self) {
        self.xywh.move_up();
        self.update_photo().ok();
    }

    pub fn xywh_move_down(&mut self) {
        self.xywh.move_down();
        self.update_photo().ok();
    }
    pub fn xywh_zoom_in(&mut self) {
        self.xywh.zoom_in();
        self.update_photo().ok();
    }
    pub fn xywh_zoom_out(&mut self) {
        self.xywh.zoom_out();
        self.update_photo().ok();
    }
    pub fn xywh_toggle_hide(&mut self) {
        self.clear_photo().ok();
        let mut config_tui = self.config_tui.write();

        // dont save value if cli has overwritten it, but still allow runtime changing
        if let Some(current) = config_tui.coverart_hidden_overwrite {
            config_tui.coverart_hidden_overwrite = Some(!current);
            info!("Not saving coverart.hidden as it is overwritten by cli!");
        } else {
            config_tui.settings.coverart.hidden = !config_tui.settings.coverart.hidden;
        }

        drop(config_tui);
        self.update_photo().ok();
    }
    fn should_not_show_photo(&self) -> bool {
        if self.app.mounted(&Id::HelpPopup) {
            return true;
        }
        if self.app.mounted(&Id::PodcastSearchTablePopup) {
            return true;
        }

        if self.app.mounted(&Id::TagEditor(IdTagEditor::InputTitle)) {
            return true;
        }

        if self.app.mounted(&Id::YoutubeSearchTablePopup) {
            return true;
        }

        if self.app.mounted(&Id::GeneralSearchInput) {
            return true;
        }

        if self.playback.is_stopped() {
            return true;
        }

        if self.app.mounted(&Id::ConfigEditor(IdConfigEditor::Header)) {
            return true;
        }

        false
    }

    /// Overwrite whatever is currently drawn in the Cover panel with a blank opaque image.
    ///
    /// Only meaningful in the `TreeView` layout (where `cover_area` is set); other layouts
    /// don't have a dedicated Cover panel to correspond to, so this is a no-op there.
    fn clear_cover_pixels(&mut self) -> Result<()> {
        if self.cover_area.is_none() {
            return Ok(());
        }
        // No alpha channel, so it can never be treated as "transparent, skip drawing".
        let blank = DynamicImage::new_rgb8(2, 2);
        self.show_image(&blank)
    }

    /// Get and show a image for the current playing media
    ///
    /// Requires that the current thread has a entered runtime
    #[allow(clippy::cast_possible_truncation)]
    pub fn update_photo(&mut self) -> Result<()> {
        if self.config_tui.read().get_coverart_hidden() {
            return Ok(());
        }
        self.clear_photo()?;
        // Whether a real image is currently on screen and needs erasing if this call ends
        // up not finding one. Only true if the *previous* call actually drew one - if we
        // were already showing the placeholder, there is nothing to erase.
        let had_real_cover = !self.cover_placeholder;
        // Assume no cover until one of the branches below actually shows one; this also
        // covers "should_not_show_photo" / "no track" / "no picture found" cases uniformly,
        // so the Cover panel never keeps showing a stale previous track's art.
        self.cover_placeholder = true;

        if self.should_not_show_photo() {
            if had_real_cover {
                self.clear_cover_pixels()?;
            }
            return Ok(());
        }

        let Some(track) = self.playback.current_track() else {
            if had_real_cover {
                self.clear_cover_pixels()?;
            }
            return Ok(());
        };

        match track.inner() {
            MediaTypes::Track(track_data) => {
                let res = match track.get_picture() {
                    Ok(v) => v,
                    Err(err) => {
                        error!(
                            "Getting the cover for \"{}\" failed! Error: {}",
                            track_data.path().display(),
                            err
                        );
                        if had_real_cover {
                            self.clear_cover_pixels()?;
                        }
                        return Ok(());
                    }
                };
                if let Some(picture) = res
                    && let Ok(image) = image::load_from_memory(picture.data())
                {
                    // A full image draw already replaces whatever pixels were there before,
                    // so there is no need to blank first - doing so would just flash black
                    // in between every track change that has cover art.
                    self.show_image(&image)?;
                    self.cover_placeholder = false;
                    return Ok(());
                }

                // No embedded picture - e.g. yt-dlp opus downloads deliberately skip
                // `--embed-thumbnail` (it corrupts the Ogg container for lofty). Fall back to
                // the sibling thumbnail file yt-dlp wrote alongside the track instead.
                if let Some(thumb_path) = sidecar_thumbnail_path(track_data.path())
                    && let Ok(image) = image::open(&thumb_path)
                {
                    self.show_image(&image)?;
                    self.cover_placeholder = false;
                    return Ok(());
                }

                // Protocol renderers (sixel/kitty/iterm) composite graphics independently of
                // ratatui's own text-cell redraw, so a leftover image from the previous
                // track isn't actually erased just because a "TwT" placeholder gets drawn as
                // text on top of it (see `view_layout_treeview`) - only reachable here once
                // we know for sure there is no replacement image to draw instead.
                if had_real_cover {
                    self.clear_cover_pixels()?;
                }
            }
            MediaTypes::Radio(_radio_track_data) => {
                if had_real_cover {
                    self.clear_cover_pixels()?;
                }
            }
            MediaTypes::Podcast(podcast_track_data) => {
                let url = {
                    if let Some(episode_photo_url) = podcast_track_data.image_url() {
                        episode_photo_url.to_string()
                    } else if let Some(pod_photo_url) =
                        self.podcast_get_album_photo_by_url(podcast_track_data.url())
                    {
                        pod_photo_url
                    } else {
                        if had_real_cover {
                            self.clear_cover_pixels()?;
                        }
                        return Ok(());
                    }
                };

                if url.is_empty() {
                    if had_real_cover {
                        self.clear_cover_pixels()?;
                    }
                    return Ok(());
                }
                let tx = self.tx_to_main.clone();

                // Leave whatever is currently shown up until the async fetch below resolves
                // (via `CoverDLResult`) instead of blanking now - avoids a flash for the
                // common case where the fetch succeeds.
                Handle::current().spawn(Self::fetch_podcast_image(tx, url));
            }
        }

        Ok(())
    }

    /// Fetch the given url as a image and send events when done or error.
    async fn fetch_podcast_image(tx: TxToMain, url: String) {
        match reqwest::get(&url).await {
            Ok(result) => {
                if result.status() != reqwest::StatusCode::OK {
                    tx.send(Msg::Xywh(XYWHMsg::CoverDLResult(
                        CoverDLResult::FetchPhotoErr(format!(
                            "Error non-OK Status code: {}",
                            result.status()
                        )),
                    )))
                    .ok();
                    return;
                }

                let cursor = {
                    let bytes = match result.bytes().await {
                        Ok(v) => v,
                        Err(err) => {
                            tx.send(Msg::Xywh(XYWHMsg::CoverDLResult(
                                CoverDLResult::FetchPhotoErr(format!(
                                    "Error in reqest::Response::bytes: {err}"
                                )),
                            )))
                            .ok();
                            return;
                        }
                    };

                    std::io::Cursor::new(bytes)
                };

                let image = match image::ImageReader::new(cursor).with_guessed_format() {
                    Ok(v) => v,
                    Err(err) => {
                        let _ = tx.send(Msg::Xywh(XYWHMsg::CoverDLResult(
                            CoverDLResult::FetchPhotoErr(format!(
                                "Failed to get a valid format for downloaded image: {err}"
                            )),
                        )));
                        return;
                    }
                };

                match image.decode() {
                    Ok(image) => {
                        let image_wrapper = ImageWrapper { data: image };
                        tx.send(Msg::Xywh(XYWHMsg::CoverDLResult(
                            CoverDLResult::FetchPhotoSuccess(image_wrapper),
                        )))
                        .ok()
                    }
                    Err(e) => tx
                        .send(Msg::Xywh(XYWHMsg::CoverDLResult(
                            CoverDLResult::FetchPhotoErr(format!(
                                "Decoding downloaded image failed: {e}"
                            )),
                        )))
                        .ok(),
                }
            }
            Err(e) => tx
                .send(Msg::Xywh(XYWHMsg::CoverDLResult(
                    CoverDLResult::FetchPhotoErr(format!("Error in ureq get: {e}")),
                )))
                .ok(),
        };
    }

    #[allow(clippy::cast_possible_truncation, clippy::unnecessary_wraps)]
    pub fn show_image(&mut self, img: &DynamicImage) -> Result<()> {
        // In the TreeView layout the cover art gets a dedicated bordered panel
        // (top-right, 30%h x 20%w) instead of a floating xywh-percentage overlay.
        if let Some(area) = self.cover_area {
            return self.show_image_fixed(img, area);
        }

        #[allow(unused_variables)]
        let xywh = self.xywh.update_size(img)?;

        // error!("{:?}", self.viuer_supported);
        match self.viuer_supported {
            ViuerSupported::NotSupported => {
                #[cfg(all(feature = "cover-ueberzug", not(target_os = "windows")))]
                let drew_with_ueberzug = if let Some(instance) = self.ueberzug_instance.as_mut() {
                    let mut cache_file = dirs::cache_dir().unwrap_or_else(std::env::temp_dir);
                    cache_file.push("termusic");
                    if !cache_file.exists() {
                        std::fs::create_dir_all(&cache_file)?;
                    }
                    cache_file.push("termusic_cover.jpg");
                    img.save(&cache_file)?;
                    if !cache_file.exists() {
                        anyhow::bail!("cover file is not saved correctly");
                    }
                    if let Some(file) = cache_file.as_path().to_str() {
                        instance.draw_cover_ueberzug(file, &xywh, false)?;
                    }
                    true
                } else {
                    false
                };
                #[cfg(not(all(feature = "cover-ueberzug", not(target_os = "windows"))))]
                let drew_with_ueberzug = false;

                // No protocol-specific rendering (sixel/kitty/iterm2/ueberzug) available
                // (e.g. plain Windows Terminal/conhost): fall back to viuer's built-in ANSI
                // truecolor half-block renderer, which needs no special terminal support
                // beyond 24-bit color.
                if !drew_with_ueberzug {
                    let config = viuer::Config {
                        transparent: true,
                        absolute_offset: true,
                        x: xywh.x as u16,
                        y: xywh.y as i16,
                        width: Some(xywh.width),
                        height: None,
                        ..viuer::Config::default()
                    };
                    viuer::print(img, &config).context("viuer::print")?;
                }
            }
            #[cfg(any(
                feature = "cover-viuer-iterm",
                feature = "cover-viuer-kitty",
                feature = "cover-viuer-sixel"
            ))]
            _ => {
                let config = viuer::Config {
                    transparent: true,
                    absolute_offset: true,
                    x: xywh.x as u16,
                    y: xywh.y as i16,
                    width: Some(xywh.width),
                    height: None,
                    // Force the specific protocol we probed for earlier
                    #[cfg(feature = "cover-viuer-iterm")]
                    use_iterm: self.viuer_supported == ViuerSupported::ITerm,
                    #[cfg(feature = "cover-viuer-kitty")]
                    use_kitty: self.viuer_supported == ViuerSupported::Kitty,
                    #[cfg(feature = "cover-viuer-sixel")]
                    use_sixel: self.viuer_supported == ViuerSupported::Sixel,
                    ..viuer::Config::default()
                };
                viuer::print(img, &config).context("viuer::print")?;
            }
        }

        Ok(())
    }

    /// Fit `img` into a fixed screen-space rect (the `TreeView` layout's cover panel),
    /// instead of the floating xywh-percentage placement used by [`Self::show_image`].
    #[allow(
        clippy::unnecessary_wraps,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn show_image_fixed(&mut self, img: &DynamicImage, area: Rect) -> Result<()> {
        // Protocol renderers (sixel/kitty/iterm) assume a fixed pixels-per-cell ratio that
        // often doesn't match the terminal's actual font metrics, leaving a visible gap
        // around the image. Overscan a bit to compensate; the plain ANSI block fallback has
        // no such mismatch (it uses real character cells 1:1), so keep it exact there.
        let overscan = if self.viuer_supported == ViuerSupported::NotSupported {
            1.0
        } else {
            1.69
        };
        let width = (f64::from(area.width) * overscan).round() as u32;
        let height = (f64::from(area.height) * overscan).round() as u32;

        let config = viuer::Config {
            transparent: true,
            absolute_offset: true,
            x: area.x,
            y: area.y.cast_signed(),
            width: Some(width),
            height: Some(height),
            #[cfg(feature = "cover-viuer-iterm")]
            use_iterm: self.viuer_supported == ViuerSupported::ITerm,
            #[cfg(feature = "cover-viuer-kitty")]
            use_kitty: self.viuer_supported == ViuerSupported::Kitty,
            #[cfg(feature = "cover-viuer-sixel")]
            use_sixel: self.viuer_supported == ViuerSupported::Sixel,
            ..viuer::Config::default()
        };
        viuer::print(img, &config).context("viuer::print")?;
        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn clear_photo(&mut self) -> Result<()> {
        match self.viuer_supported {
            #[cfg(feature = "cover-viuer-kitty")]
            ViuerSupported::Kitty => {
                self.clear_image_viuer_kitty()
                    .context("clear_photo kitty")?;
                Self::remove_temp_files()?;
            }
            #[cfg(feature = "cover-viuer-iterm")]
            ViuerSupported::ITerm => {
                self.clear_image_viuer_kitty()
                    .context("clear_photo iterm")?;
                Self::remove_temp_files()?;
            }
            #[cfg(feature = "cover-viuer-sixel")]
            ViuerSupported::Sixel => {
                self.clear_image_viuer_kitty()
                    .context("clear_photo sixel")?;
                // sixel does not use temp-files, so no cleaning necessary
            }
            ViuerSupported::NotSupported => {
                #[cfg(all(feature = "cover-ueberzug", not(target_os = "windows")))]
                if let Some(instance) = self.ueberzug_instance.as_mut() {
                    instance.clear_cover_ueberzug()?;
                }
            }
        }
        Ok(())
    }

    #[cfg(any(
        feature = "cover-viuer-iterm",
        feature = "cover-viuer-kitty",
        feature = "cover-viuer-sixel"
    ))]
    fn clear_image_viuer_kitty(&mut self) -> Result<()> {
        use tuirealm::terminal::TerminalAdapter;

        write!(self.terminal.raw_mut().backend_mut(), "\x1b_Ga=d\x1b\\")?;
        self.terminal.raw_mut().backend_mut().flush()?;
        Ok(())
    }

    #[cfg(any(feature = "cover-viuer-iterm", feature = "cover-viuer-kitty"))]
    fn remove_temp_files() -> Result<()> {
        // Clean up temp files created by `viuer`'s kitty printer to avoid
        // possible freeze because of too many temp files in the temp folder.
        // Context: https://github.com/aome510/spotify-player/issues/148
        let tmp_dir = std::env::temp_dir();
        for path in (std::fs::read_dir(tmp_dir)?).flatten() {
            let path = path.path();
            if path.display().to_string().contains(".tmp.viuer") {
                std::fs::remove_file(path)?;
            }
        }

        Ok(())
    }
}
