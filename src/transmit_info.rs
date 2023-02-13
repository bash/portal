use portal_wormhole::TransitInfo;
use std::ffi::OsStr;
use std::fmt;

pub fn transit_info_message(transit_info: &TransitInfo, filename: &OsStr) -> String {
    let filename = filename.to_string_lossy();
    format!("File \"{filename}\"{}", TransitInfoDisplay(transit_info))
}

struct TransitInfoDisplay<'a>(&'a TransitInfo);

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
