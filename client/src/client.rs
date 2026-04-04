pub struct WebRtcClient {
    _connection: RtcPeerConnection,
    data_channel: RtcDataChannel,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_close: Closure<dyn FnMut(web_sys::Event)>,
    receiver: RefCell<mpsc::UnboundedReceiver<Vec<u8>>>,
}

impl WebRtcClient {
    /*
    https://libp2p.io/guides/browser-connectivity/#webrtc
    https://libp2p.io/blog/rust-libp2p-browser-webrtc/
    https://github.com/libp2p/rust-libp2p/tree/master/examples/browser-webrtc
    https://github.com/libp2p/specs/blob/master/webrtc/webrtc-direct.md
     */
    pub async fn connect(
        server_addr: SocketAddr,
        server_fingerprint: Fingerprint,
    ) -> Result<Self, WebRtcError> {
        let (_connection, data_channel) = Self::handshake(server_addr, server_fingerprint).await?;

        let (_on_message, _on_close, receiver) = Self::start_listener(&data_channel);

        Ok(Self {
            _connection,
            data_channel,
            _on_message,
            _on_close,
            receiver: RefCell::new(receiver),
        })
    }

    async fn handshake(
        server_addr: SocketAddr,
        server_fingerprint: Fingerprint,
    ) -> Result<(RtcPeerConnection, RtcDataChannel), WebRtcError> {
        let config = web_sys::RtcConfiguration::new();

        config.set_ice_servers(&js_sys::Array::new());
        let pc = RtcPeerConnection::new_with_configuration(&config)?;

        let dc_init = web_sys::RtcDataChannelInit::new();
        dc_init.set_negotiated(true);
        dc_init.set_id(0);
        let dc = pc.create_data_channel_with_data_channel_dict("rpc", &dc_init);
        let ufrag = Ufrag::random();

        let offer = JsFuture::from(pc.create_offer()).await?;
        let offer: web_sys::RtcSessionDescription = offer.unchecked_into();
        let munged = crate::sdp::munge_offer(&offer.sdp(), ufrag);
        JsFuture::from(pc.set_local_description(&munged)).await?;

        let answer = crate::sdp::server_answer(server_addr, server_fingerprint, ufrag);
        JsFuture::from(pc.set_remote_description(&answer)).await?;

        Self::wait_for_dc_open(&dc).await?;
        dc.set_binary_type(web_sys::RtcDataChannelType::Arraybuffer);

        Ok((pc, dc))
    }

    async fn wait_for_dc_open(dc: &RtcDataChannel) -> Result<(), WebRtcError> {
        if dc.ready_state() == RtcDataChannelState::Open {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel::<()>();
        let tx = Rc::new(RefCell::new(Some(tx)));

        let notify = Closure::wrap(Box::new(move || {
            if let Some(tx) = tx.borrow_mut().take() {
                let _ = tx.send(());
            }
        }) as Box<dyn FnMut()>);

        dc.set_onopen(Some(notify.as_ref().unchecked_ref()));
        dc.set_onerror(Some(notify.as_ref().unchecked_ref()));

        let timeout = gloo_timers::future::TimeoutFuture::new(10_000);
        future::select(Box::pin(rx), Box::pin(timeout)).await;

        dc.set_onopen(None);
        dc.set_onerror(None);

        if dc.ready_state() == RtcDataChannelState::Open {
            Ok(())
        } else {
            Err(WebRtcError::BrowserApi(format!("DataChannel state: {:?}", dc.ready_state())))
        }
    }

    fn start_listener(
        dc: &RtcDataChannel,
    ) -> (
        Closure<dyn FnMut(MessageEvent)>,
        Closure<dyn FnMut(web_sys::Event)>,
        mpsc::UnboundedReceiver<Vec<u8>>,
    ) {
        let (tx, rx) = mpsc::unbounded();
        let close_tx = tx.clone();
        let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
            let Ok(buf) = event.data().dyn_into::<js_sys::ArrayBuffer>() else { return };
            let raw_bytes = js_sys::Uint8Array::new(&buf).to_vec();
            let _ = tx.unbounded_send(raw_bytes);
        }) as Box<dyn FnMut(MessageEvent)>);
        let on_close = Closure::wrap(Box::new(move |_: web_sys::Event| {
            close_tx.close_channel();
        }) as Box<dyn FnMut(web_sys::Event)>);
        dc.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        dc.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        (on_message, on_close, rx)
    }

    pub async fn recv_raw(&self) -> Option<Vec<u8>> {
        use futures::StreamExt;
        self.receiver.borrow_mut().next().await
    }

    pub fn write_all(&self, data: &[u8]) -> Result<(), WebRtcError> {
        for chunk in split_chunks(data) {
            self.data_channel.send_with_u8_array(chunk)?;
        }
        Ok(())
    }
}

use crate::WebRtcError;
use futures::channel::{mpsc, oneshot};
use futures::future;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{MessageEvent, RtcDataChannel, RtcDataChannelState, RtcPeerConnection};
use webrtc_direct_protocol::{Fingerprint, Ufrag, split_chunks};
