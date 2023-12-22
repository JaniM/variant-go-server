use dioxus::prelude::*;
use dioxus_signals::*;

#[derive(Clone, Default)]
struct WindowSize((f64, f64));

fn window_size() -> (f64, f64) {
    let window = gloo_utils::window();
    let width = window.inner_width().unwrap().as_f64().unwrap();
    let height = window.inner_height().unwrap().as_f64().unwrap();
    (width, height)
}

pub(crate) fn use_window_size_provider(cx: &ScopeState) -> (f64, f64) {
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

pub(crate) fn use_window_size(cx: &ScopeState) -> (f64, f64) {
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
    if width < 1200.0 {
        DisplayMode::Mobile
    } else {
        DisplayMode::Desktop
    }
}
