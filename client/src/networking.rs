use wasm_bindgen::prelude::*;

use wasm_bindgen::JsCast;
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

use std::cell::RefCell;

use crate::utils::{self, local_storage};
use shared::message::{ClientMessage, ServerMessage};

macro_rules! console_log {
    ($($t:tt)*) => (web_sys::console::log_1(&JsValue::from_str(&format!($($t)*))))
}

#[derive(Debug, Clone)]
struct WsHandler {
    ws: Option<WebSocket>,
}

thread_local! {
    static HANDLER: RefCell<WsHandler> = RefCell::new(WsHandler {
        ws: None
    });
}

pub fn get_token() -> Option<String> {
    local_storage().get_item("token").unwrap()
}

pub fn set_token(token: &str) {
    local_storage().set_item("token", token).unwrap();
}

fn wrap<T>(f: impl FnMut(T) + 'static) -> Closure<dyn FnMut(T)>
where
    T: wasm_bindgen::convert::FromWasmAbi + 'static,
{
    Closure::wrap(Box::new(f) as Box<dyn FnMut(T)>)
}

pub enum ServerError {
    LostConnection,

    /// This is a bit hacky. Figure out a better way?
    Clear,
}

pub fn start_websocket(
    on_msg: impl (Fn(Result<ServerMessage, ServerError>)) + Clone + 'static,
) -> Result<(), JsValue> {
    let window = web_sys::window().expect("Window not available");
    let hostname = window.location().hostname().expect("host not available");

    let host = if cfg!(feature = "local") {
        format!("ws://{}:8088/ws/", hostname)
    } else {
        format!("wss://{}/ws/", hostname)
    };

    let ws = WebSocket::new(&host)?;

    let cloned_ws = ws.clone();
    HANDLER.with(move |h| h.borrow_mut().ws = Some(cloned_ws));

    // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
    // create callback
    let cloned_on_msg = on_msg.clone();
    let onmessage_callback = wrap(move |e: MessageEvent| {
        // Handle difference Text/Binary,...
        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let array = js_sys::Uint8Array::new(&abuf);
            let msg = match serde_cbor::from_slice::<ServerMessage>(&array.to_vec()) {
                Ok(v) => v,
                Err(e) => {
                    console_log!("{:?}", e);
                    return;
                }
            };
            cloned_on_msg(Ok(msg));
        } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
            console_log!("message event, received blob: {:?}", blob);
        } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
            console_log!("message event, received Text: {:?}", txt);
        } else {
            console_log!("message event, received Unknown: {:?}", e.data());
        }
    });
    // set message event handler on WebSocket
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    // forget the callback to keep it alive
    onmessage_callback.forget();

    let cloned_on_msg = on_msg.clone();
    let onerror_callback = wrap(move |e: ErrorEvent| {
        console_log!("error event: {:?}", e);
        cloned_on_msg(Err(ServerError::LostConnection));
        // let _ = start_websocket(cloned_on_msg.clone());
    });
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    let cloned_on_msg = on_msg.clone();
    let onclose_callback = wrap(move |_: CloseEvent| {
        cloned_on_msg(Err(ServerError::LostConnection));
        let _ = start_websocket(cloned_on_msg.clone());
    });
    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();

    let onopen_callback = wrap(move |_: JsValue| {
        console_log!("socket opened");
        on_msg(Err(ServerError::Clear));

        // TODO: these should not be here
        send(ClientMessage::GetGameList);
        send(ClientMessage::Identify {
            token: get_token(),
            nick: None,
        });

        // TODO: use a proper router?

        let hash = utils::get_hash();
        if hash.starts_with('#') {
            if let Ok(id) = hash[1..].parse::<u32>() {
                send(ClientMessage::JoinGame(id));
            }
        }
    });
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    Ok(())
}

pub fn send(msg: impl Into<ClientMessage>) {
    HANDLER.with(|h| {
        let handler = h.borrow();
        let vec = serde_cbor::to_vec(&msg.into()).expect("cbor serialization failed");
        match handler
            .ws
            .as_ref()
            .expect("ws not initialized")
            .send_with_u8_array(&vec)
        {
            Ok(_) => {}
            Err(err) => console_log!("error sending message: {:?}", err),
        };
    });
}
