use wasm_bindgen::prelude::*;

use wasm_bindgen::JsCast;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

use std::cell::RefCell;

use crate::message::{ClientMessage, ServerMessage};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
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

pub fn local_storage() -> web_sys::Storage {
    let window = web_sys::window().expect("Window not available");
    window.local_storage().unwrap().unwrap()
}

pub fn get_token() -> Option<String> {
    local_storage().get_item("token").unwrap()
}

pub fn set_token(token: &str) {
    local_storage().set_item("token", token).unwrap();
}

pub fn start_websocket(on_msg: impl Fn(ServerMessage) -> () + 'static) -> Result<(), JsValue> {
    // Connect to an echo server
    let ws = WebSocket::new("ws://localhost:8088/ws/")?;
    let cloned_ws = ws.clone();
    HANDLER.with(move |h| h.borrow_mut().ws = Some(cloned_ws));

    // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
    // create callback
    let cloned_ws = ws.clone();
    let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        // Handle difference Text/Binary,...
        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            console_log!("message event, received arraybuffer: {:?}", abuf);
            let array = js_sys::Uint8Array::new(&abuf);
            let len = array.byte_length() as usize;
            console_log!("Arraybuffer received {}bytes: {:?}", len, array.to_vec());
            let msg = match serde_cbor::from_slice::<ServerMessage>(&array.to_vec()) {
                Ok(v) => v,
                Err(e) => {
                    console_log!("{:?}", e);
                    return;
                }
            };
            console_log!("{:?}", msg);
            on_msg(msg);
        } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
            console_log!("message event, received blob: {:?}", blob);
        } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
            console_log!("message event, received Text: {:?}", txt);
        } else {
            console_log!("message event, received Unknown: {:?}", e.data());
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    // set message event handler on WebSocket
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    // forget the callback to keep it alive
    onmessage_callback.forget();

    let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
        console_log!("error event: {:?}", e);
    }) as Box<dyn FnMut(ErrorEvent)>);
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    let onopen_callback = Closure::wrap(Box::new(move |_| {
        console_log!("socket opened");
        send(ClientMessage::GetGameList);
        send(ClientMessage::Identify {
            token: get_token(),
            nick: None,
        });
    }) as Box<dyn FnMut(JsValue)>);
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    Ok(())
}

pub fn send(msg: ClientMessage) {
    HANDLER.with(|h| {
        let handler = h.borrow();
        let mut vec = serde_cbor::to_vec(&msg).expect("cbor serialization failed");
        match handler
            .ws
            .as_ref()
            .expect("ws not initialized")
            .send_with_u8_array(&mut vec)
        {
            Ok(_) => console_log!("binary message successfully sent"),
            Err(err) => console_log!("error sending message: {:?}", err),
        };
    });
}
