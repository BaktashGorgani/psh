use tracing::{debug, info};

use crate::registry::{Entry, Registry};

#[derive(Debug, Clone)]
pub enum Parsed {
    Default {
        command: String,
    },
    Entry {
        name: String,
        entry: Entry,
        command: String,
    },
}

pub fn parse(registry: &Registry, input: &str) -> Parsed {
    let max_len = registry.max_name_len();
    debug!(input = input, max_len = max_len, "parse start");
    if max_len == 0 {
        info!("parse default_no_names");
        return Parsed::Default {
            command: input.trim().to_string(),
        };
    }

    let mut chars_seen = 0usize;
    for (byte_idx, ch) in input.char_indices() {
        if ch == ':' {
            let name = &input[..byte_idx];
            let rest = input[byte_idx + ch.len_utf8()..].trim_start();
            match registry.get_entry(name) {
                Some(entry) => {
                    info!(name = name, "parse matched");
                    return Parsed::Entry {
                        name: name.to_string(),
                        entry,
                        command: rest.to_string(),
                    };
                }
                None => {
                    return Parsed::Default {
                        command: input.trim().to_string(),
                    };
                }
            }
        }
        chars_seen += 1;
        if chars_seen > max_len {
            info!("parse default_exceeded_bound");
            return Parsed::Default {
                command: input.trim().to_string(),
            };
        }
    }

    info!("parse default_no_colon_within_bound");
    Parsed::Default {
        command: input.trim().to_string(),
    }
}
