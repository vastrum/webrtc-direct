#[derive(thiserror::Error, Debug)]
pub enum WebRtcError {
    #[error("browser API unavailable: {0}")]
    BrowserApi(String),
    #[error("js: {0}")]
    Js(String),
}
impl From<JsValue> for WebRtcError {
    fn from(val: JsValue) -> Self {
        WebRtcError::Js(format!("{:?}", val))
    }
}

use wasm_bindgen::prelude::*;
