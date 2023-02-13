use lazy_static::lazy_static;
use magic_wormhole::transit::{RelayHint, DEFAULT_RELAY_SERVER};

lazy_static! {
    pub static ref RELAY_HINTS: Vec<RelayHint> = relay_hints();
}

fn relay_hints() -> Vec<RelayHint> {
    let hint = RelayHint::from_urls(None, [DEFAULT_RELAY_SERVER.parse().unwrap()]).unwrap();
    vec![hint]
}
