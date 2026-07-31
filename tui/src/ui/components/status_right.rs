use termusiclib::config::TuiOverlay;
use tui_realm_stdlib::components::Label;
use tuirealm::component::{AppComponent, Component};
use tuirealm::event::Event;
use tuirealm::props::HorizontalAlignment;

use crate::ui::model::UserEvent;
use crate::ui::msg::Msg;

/// Right 30% of the status bar: volume / speed / shuffle / repeat readout.
#[derive(Component)]
pub struct StatusRight {
    component: Label,
}

impl StatusRight {
    pub fn new(config: &TuiOverlay) -> Self {
        Self {
            component: Label::default()
                .foreground(config.settings.theme.progress_foreground())
                .background(config.settings.theme.progress_background())
                .alignment_horizontal(HorizontalAlignment::Center)
                .text("Vol: [          ] | Speed: 1.0"),
        }
    }
}

impl AppComponent<Msg, UserEvent> for StatusRight {
    fn on(&mut self, _ev: &Event<UserEvent>) -> Option<Msg> {
        None
    }
}
