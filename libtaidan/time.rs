/// Setting date & time.

pub fn set_datetime() -> crate::Res<()> {
    crate::steps::cmd(
        "not_found",
        &[
            "-q",
            "server 162.159.200.1 iburst",
            "server 162.159.200.123 iburst",
            "server time.cloudflare.com iburst",
            "server pool.ntp.org iburst",
        ],
    )
}
