use crate::{Progress, RequestRepaint};
use lazy_static::lazy_static;
use magic_wormhole::transit::{RelayHint, TransitInfo, DEFAULT_RELAY_SERVER};
use single_value_channel as svc;
use std::net::SocketAddr;
use url::Url;

lazy_static! {
    pub static ref RELAY_HINTS: Vec<RelayHint> = relay_hints();
}

pub trait TransitHandler: FnOnce(TransitInfo, SocketAddr) {}

impl<F> TransitHandler for F where F: FnOnce(TransitInfo, SocketAddr) {}

pub trait ProgressHandler: FnMut(u64, u64) + 'static {}

impl<F> ProgressHandler for F where F: FnMut(u64, u64) + 'static {}

pub fn transit_handler(
    sender: ::oneshot::Sender<TransitInfo>,
    mut request_repaint: impl RequestRepaint,
) -> impl TransitHandler {
    move |transit_info, _| {
        _ = sender.send(transit_info);
        request_repaint();
    }
}

pub fn progress_handler(
    updater: svc::Updater<Progress>,
    mut request_repaint: impl RequestRepaint,
) -> impl ProgressHandler {
    move |value, total| {
        _ = updater.update(Progress { value, total });
        request_repaint();
    }
}

fn relay_hints() -> Vec<RelayHint> {
    let hint = RelayHint::from_urls(None, [default_relay_server()])
        .expect("constant relay hints should be valid");
    vec![hint]
}

fn default_relay_server() -> Url {
    DEFAULT_RELAY_SERVER
        .parse()
        .expect("constant URL should be valid")
}
