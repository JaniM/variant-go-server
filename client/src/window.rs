use dioxus::prelude::*;
use dioxus_signals::*;

use crate::config;

#[derive(Clone, Default)]
struct WindowSize((i32, i32));

fn window_size() -> (i32, i32) {
    let window = gloo_utils::window();
    let document = window.document().unwrap();
    let element = document.document_element().unwrap();
    let width = element.client_width();
    let height = element.client_height();
    (width, height)
}

pub(crate) fn use_window_size_provider(cx: &ScopeState) {
    let size = *use_context_provider(cx, || Signal::new(WindowSize(window_size())));
    cx.use_hook(move || {
        let window = gloo_utils::window();
        gloo_events::EventListener::new(&window, "resize", move |_| {
            size.write().0 = window_size();
        })
    });
}

pub(crate) fn use_window_size(cx: &ScopeState) -> (i32, i32) {
    let size = use_context::<Signal<WindowSize>>(cx).expect("WindowSize not set up");
    size.read().0
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DisplayMode {
    Desktop(bool),
    Mobile,
}

impl DisplayMode {
    pub(crate) fn class(self) -> &'static str {
        match self {
            DisplayMode::Desktop(true) => "desktop small",
            DisplayMode::Desktop(false) => "desktop large",
            DisplayMode::Mobile => "mobile",
        }
    }

    pub(crate) fn is_mobile(self) -> bool {
        matches!(self, Self::Mobile)
    }

    pub(crate) fn is_desktop(self) -> bool {
        matches!(self, Self::Desktop(_))
    }

    pub(crate) fn is_small_desktop(self) -> bool {
        matches!(self, Self::Desktop(true))
    }
}

pub(crate) fn use_display_mode(cx: &ScopeState) -> DisplayMode {
    let (width, height) = use_window_size(cx);
    let sidebar_size = config::SIDEBAR_SIZE;
    if width < height + sidebar_size {
        DisplayMode::Mobile
    } else if width < height + sidebar_size * 2 {
        DisplayMode::Desktop(true)
    } else {
        DisplayMode::Desktop(false)
    }
}
