use crate::RequestRepaint;
use lazy_static::lazy_static;
use magic_wormhole::transit::{RelayHint, TransitInfo, DEFAULT_RELAY_SERVER};
use std::net::SocketAddr;

lazy_static! {
    pub static ref RELAY_HINTS: Vec<RelayHint> = relay_hints();
}

pub trait TransitHandler : FnOnce(TransitInfo, SocketAddr) {}

impl<F> TransitHandler for F where F: FnOnce(TransitInfo, SocketAddr) {}

pub fn transit_handler(
    sender: ::oneshot::Sender<TransitInfo>,
    mut request_repaint: impl RequestRepaint,
) -> impl TransitHandler {
    move |transit_info, _| {
        _ = sender.send(transit_info);
        request_repaint();
    }
}

fn relay_hints() -> Vec<RelayHint> {
    let hint = RelayHint::from_urls(None, [DEFAULT_RELAY_SERVER.parse().unwrap()]).unwrap();
    vec![hint]
}
