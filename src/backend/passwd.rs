//! Parse /etc/passwd

use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use itertools::Itertools;

#[must_use]
pub fn user_already_exists() -> bool {
    // if cfg!(debug_assertions) {
    //     tracing::info!("skipping user_already_exists() check since debug_assertions is on");
    //     return false;
    // }
    let Ok(f) =
        File::open("/etc/passwd").inspect_err(|e| tracing::error!(?e, "cannot open /etc/passwd"))
    else {
        return false;
    };
    let reader = BufReader::new(f);
    reader.lines().any(|l| {
        l.is_ok_and(|l| {
            let Some([user, _, uid, _]) = l.splitn(4, ':').collect_array() else {
                tracing::error!(?l, "cannot parse line in /etc/passwd");
                return false;
            };
            let Ok(uid) = uid
                .parse::<usize>()
                .inspect_err(|e| tracing::error!(?e, ?uid, "cannot parse uid"))
            else {
                return false;
            };
            if (1000..60000).contains(&uid) {
                tracing::info!(?user, ?uid, "found normal user");
                return true;
            }
            false
        })
    })
}
