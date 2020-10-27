use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

pub fn get_hash() -> String {
    let window = web_sys::window().expect("Window not available");
    window.location().hash().expect("url hash not available")
}

pub fn set_hash(hash: &str) {
    let window = web_sys::window().expect("Window not available");
    window
        .location()
        .set_hash(hash)
        .expect("url hash not available");
}

pub fn local_storage() -> web_sys::Storage {
    let window = web_sys::window().expect("Window not available");
    window.local_storage().unwrap().unwrap()
}

pub fn download_file(name: &str, data: &str) -> Result<(), JsValue> {
    use web_sys::{Blob, BlobPropertyBag, HtmlElement, Url};
    let document = web_sys::window().unwrap().document().unwrap();

    let mut props = BlobPropertyBag::new();
    props.type_("text/plain");

    let blob =
        Blob::new_with_str_sequence_and_options(&JsValue::from_serde(&[data]).unwrap(), &props)?;
    let link = document.create_element("a")?.dyn_into::<HtmlElement>()?;
    link.set_attribute("href", Url::create_object_url_with_blob(&blob)?.as_str())?;
    link.set_attribute("download", name)?;

    let body = document.body().unwrap();
    body.append_child(&link)?;
    link.click();
    body.remove_child(&link)?;

    Ok(())
}

#[macro_export]
macro_rules! if_html {
    (let $pat:pat = $cond:expr => $($body:tt)+) => {
        if let $pat = $cond {
            html!($($body)+)
        } else {
            html!()
        }
    };
    ($cond:expr => $($body:tt)+) => {
        if $cond {
            html!($($body)+)
        } else {
            html!()
        }
    };
}
