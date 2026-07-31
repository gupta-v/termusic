use termusiclib::config::SharedTuiSettings;
use termusiclib::track::MediaTypes;
use tui_realm_stdlib::components::Textarea;
use tuirealm::component::{AppComponent, Component};
use tuirealm::event::Event;
use tuirealm::props::{
    AttrValue, Attribute, BorderType, Borders, HorizontalAlignment, LineStatic, Style, TextStatic,
    Title,
};

use crate::ui::Model;
use crate::ui::ids::Id;
use crate::ui::model::UserEvent;
use crate::ui::msg::Msg;

/// Small read-only panel showing name / album / artist of the current track.
#[derive(Component)]
pub struct TrackDetails {
    component: Textarea,
}

impl TrackDetails {
    pub fn new(config: &SharedTuiSettings) -> Self {
        let component = {
            let config = config.read();
            Textarea::default()
                .borders(
                    Borders::default()
                        .color(config.settings.theme.lyric_border())
                        .modifiers(BorderType::Rounded),
                )
                .background(config.settings.theme.lyric_background())
                .foreground(config.settings.theme.lyric_foreground())
                .inactive(Style::new().bg(config.settings.theme.lyric_background()))
                .title(Title::from(" Details ").alignment(HorizontalAlignment::Left))
                .text_rows(TextStatic::from("No track is playing"))
        };

        Self { component }
    }
}

impl AppComponent<Msg, UserEvent> for TrackDetails {
    fn on(&mut self, _ev: &Event<UserEvent>) -> Option<Msg> {
        None
    }
}

impl Model {
    /// Refresh the [`TrackDetails`] panel from the current track.
    ///
    /// Needs to be run on:
    /// - track change
    /// - running status change
    pub fn track_details_update(&mut self) {
        let text = if self.playback.is_stopped() {
            TextStatic::from("Stopped.")
        } else if let Some(track) = self.playback.current_track() {
            match track.inner() {
                MediaTypes::Track(track_data) => TextStatic::from_iter([
                    LineStatic::styled(
                        track.title().unwrap_or("Unknown Title").to_string(),
                        Style::new().bold(),
                    ),
                    LineStatic::from(format!(
                        "Album: {}",
                        track_data.album().unwrap_or("Unknown Album")
                    )),
                    LineStatic::from(format!(
                        "Artist: {}",
                        track.artist().unwrap_or("Unknown Artist")
                    )),
                ]),
                MediaTypes::Radio(_) => TextStatic::from("Live Radio"),
                MediaTypes::Podcast(podcast_data) => TextStatic::from_iter([
                    LineStatic::styled(
                        track.title().unwrap_or("Unknown Episode").to_string(),
                        Style::new().bold(),
                    ),
                    LineStatic::from(format!("Podcast: {}", podcast_data.url())),
                ]),
            }
        } else {
            TextStatic::from("No track is playing")
        };

        let _ = self
            .app
            .attr(&Id::TrackDetails, Attribute::Text, AttrValue::Text(text));
    }
}
