use std::ops::Div;
use std::time::Duration;

use termusiclib::config::TuiOverlay;
use termusiclib::config::v2::server::LoopMode;
use termusiclib::player::RunningStatus;
use termusiclib::track::DurationFmtShort;
use tui_realm_stdlib::components::Gauge;
use tuirealm::component::AppComponent;
use tuirealm::component::Component;
use tuirealm::event::Event;
use tuirealm::props::AttrValue;
use tuirealm::props::Attribute;
use tuirealm::props::HorizontalAlignment;
use tuirealm::props::Style;
use tuirealm::props::Title;
use tuirealm::props::{BorderType, Borders, PropPayload, PropValue};

use crate::ui::Model;
use crate::ui::ids::Id;
use crate::ui::model::UserEvent;
use crate::ui::msg::Msg;

#[derive(Component)]
pub struct Progress {
    component: Gauge,
}

impl Progress {
    #[allow(clippy::cast_precision_loss)]
    pub fn new(config: &TuiOverlay) -> Self {
        Self {
            component: Gauge::default()
                .borders(
                    Borders::default()
                        .color(config.settings.theme.progress_border())
                        .modifiers(BorderType::Rounded),
                )
                .background(config.settings.theme.progress_background())
                .foreground(config.settings.theme.progress_foreground())
                .inactive(Style::new().fg(config.settings.theme.progress_foreground()))
                .label("Progress")
                .title(
                    Title::from(
                        "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500} Stopped ",
                    )
                    .alignment(HorizontalAlignment::Left),
                )
                .progress(0.0),
        }
    }
}

impl AppComponent<Msg, UserEvent> for Progress {
    fn on(&mut self, _ev: &Event<UserEvent>) -> Option<Msg> {
        None
    }
}

/// `1` when the loop mode currently shuffles track order, `0` otherwise.
fn shuffle_str(loop_mode: LoopMode) -> &'static str {
    if loop_mode == LoopMode::Random {
        "1"
    } else {
        "0"
    }
}

/// Repeat state as a 2-bit binary string: `0` = no repeat, `1` = repeat single track,
/// `10` = repeat playlist (binary for 2, not decimal ten).
fn repeat_str(loop_mode: LoopMode) -> &'static str {
    match loop_mode {
        LoopMode::Track => "1",
        LoopMode::Playlist => "10",
        LoopMode::Random | LoopMode::PlaylistOnce => "0",
    }
}

/// Render volume (0-100) as a fixed-width `[=====     ]` ASCII bar.
fn volume_bar(volume: u16, width: usize) -> String {
    let filled = (usize::from(volume).min(100) * width) / 100;
    format!("[{}{}]", "=".repeat(filled), " ".repeat(width - filled))
}

/// Left (70%) status text: just what's playing.
fn title_format(status: RunningStatus, title: Option<&str>) -> String {
    const LEFT_PAD: &str = "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}"; // matches the rounded border's line character, instead of blank space
    match (status, title) {
        (RunningStatus::Running, Some(title)) => format!("{LEFT_PAD} Playing: {title:.30} "),
        (RunningStatus::Paused, Some(title)) => format!("{LEFT_PAD} Paused: {title:.30} "),
        (RunningStatus::Stopped, _) | (_, None) => format!("{LEFT_PAD} Stopped "),
    }
}

/// Right (30%, "Controls") status text: speed/shuffle/repeat/volume, single line.
#[allow(clippy::cast_precision_loss)] // speed is never realisitcally expected to be above i16::MAX
fn status_right_format(volume: u16, speed: i32, loop_mode: LoopMode) -> String {
    let shuffle = shuffle_str(loop_mode);
    let repeat = repeat_str(loop_mode);
    let volume = volume_bar(volume, 10);

    format!(
        "Speed:{:^.1}x | Shuf:{} | Rpt:{} | Vol:{}",
        speed as f32 / 10.0,
        shuffle,
        repeat,
        volume,
    )
}

impl Model {
    pub fn progress_reload(&mut self) {
        assert!(
            self.app
                .remount(
                    Id::Progress,
                    Box::new(Progress::new(&self.config_tui.read())),
                    Vec::new()
                )
                .is_ok()
        );
        self.progress_update_title();
    }

    /// Update the [`Progress`] component's title.
    ///
    /// This needs to be run if one of the following changes:
    /// - volume
    /// - speed
    /// - gapless
    /// - running status
    /// - moving onto / off a podcast track
    pub fn progress_update_title(&mut self) {
        let config_server = self.config_server.read();
        let player = &config_server.settings.player;

        let title = self.playback.current_track().map(|track| {
            track
                .title()
                .map_or_else(|| track.id_str().into_owned(), ToString::to_string)
        });
        let progress_title = title_format(self.playback.status(), title.as_deref());
        let status_right = status_right_format(player.volume, player.speed, player.loop_mode);

        drop(config_server);
        let _ = self.app.attr(
            &Id::Progress,
            Attribute::Title,
            AttrValue::Title(Title::from(progress_title).alignment(HorizontalAlignment::Left)),
        );
        let _ = self.app.attr(
            &Id::StatusRight,
            Attribute::Text,
            AttrValue::String(status_right),
        );

        self.force_redraw();
    }

    /// Handle progress updates.
    ///
    /// Updates all places where progress updates need to be populated to.
    #[allow(clippy::cast_precision_loss)]
    pub fn progress_update(&mut self, time_pos: Option<Duration>, total_duration: Duration) {
        let time_pos = time_pos.unwrap_or_default();

        self.playback.set_current_track_pos(time_pos);

        let progress = if time_pos.as_millis() > 0 && total_duration.as_millis() > 0 {
            (time_pos.as_millis() as f64).div(total_duration.as_millis() as f64)
        } else {
            0.0
        };

        let progress = progress.clamp(0.0, 1.0);

        self.progress_set(progress, total_duration);
        self.lyric_update();
    }

    /// Set the progress bar text.
    fn progress_set(&mut self, mut progress: f64, total_duration: Duration) {
        let text = if self.playback.is_stopped() {
            progress = 0.0;
            DurationFmtShort::fmt_empty().to_string()
        } else if total_duration.is_zero() {
            format!("{}", DurationFmtShort(self.playback.current_track_pos()))
        } else {
            format!(
                "{}    -    {}",
                DurationFmtShort(self.playback.current_track_pos()),
                DurationFmtShort(total_duration),
            )
        };

        let _ = self.app.attr(
            &Id::Progress,
            Attribute::Value,
            AttrValue::Payload(PropPayload::Single(PropValue::F64(progress))),
        );
        let _ = self
            .app
            .attr(&Id::Progress, Attribute::Text, AttrValue::String(text));
    }
}
