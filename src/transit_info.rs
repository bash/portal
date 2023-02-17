use portal_wormhole::TransitInfo;

use std::fmt;

pub struct TransitInfoDisplay<'a>(pub &'a TransitInfo);

impl<'a> fmt::Display for TransitInfoDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TransitInfo::*;
        match self.0 {
            Direct => write!(f, " via direct transfer"),
            Relay { name: None } => write!(f, " via relay"),
            Relay { name: Some(relay) } => write!(f, " via relay \"{relay}\""),
            _ => Ok(()),
        }
    }
}
