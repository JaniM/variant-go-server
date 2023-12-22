use dioxus::prelude::*;
use dioxus_signals::*;

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

pub(crate) fn use_window_size_provider(cx: &ScopeState) -> (i32, i32) {
    let size = *use_context_provider(cx, || Signal::new(WindowSize(window_size())));
    cx.use_hook(move || {
        let window = gloo_utils::window();
        gloo_events::EventListener::new(&window, "resize", move |_| {
            size.write().0 = window_size();
        })
    });
    // For some reason this doesn't compile without an assignment.
    let x = size.read().0;
    x
}

pub(crate) fn use_window_size(cx: &ScopeState) -> (i32, i32) {
    let size = use_context::<Signal<WindowSize>>(cx).expect("WindowSize not set up");
    size.read().0
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DisplayMode {
    Desktop,
    Mobile,
}

impl DisplayMode {
    pub(crate) fn class(self) -> &'static str {
        match self {
            DisplayMode::Desktop => "desktop",
            DisplayMode::Mobile => "mobile",
        }
    }
}

pub(crate) fn use_display_mode(cx: &ScopeState) -> DisplayMode {
    let (width, _) = use_window_size(cx);
    if width < 1200 {
        DisplayMode::Mobile
    } else {
        DisplayMode::Desktop
    }
}
