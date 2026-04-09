use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use webrtc_direct_client::{FramedClient, Fingerprint, WebRtcClient};

thread_local! {
    static CLIENT: RefCell<Option<Rc<FramedClient>>> = RefCell::new(None);
}

#[wasm_bindgen]
pub async fn connect(port: u16, fingerprint: String) -> Result<(), JsError> {
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let fp = Fingerprint::from_hex(fingerprint);
    let raw = WebRtcClient::connect(addr, fp)
        .await
        .map_err(|e| JsError::new(&format!("{e}")))?;
    let client = Rc::new(FramedClient::new(raw));
    CLIENT.with(|c| *c.borrow_mut() = Some(client));
    Ok(())
}

#[wasm_bindgen]
pub async fn send_echo(message: String) -> Result<String, JsError> {
    let Some(client) = CLIENT.with(|c| c.borrow().clone()) else {
        return Err(JsError::new("not connected"));
    };
    client.send(message.as_bytes()).map_err(|e| JsError::new(&format!("{e}")))?;
    let response = client.recv().await.ok_or(JsError::new("connection closed"))?;
    Ok(String::from_utf8(response).unwrap())
}
