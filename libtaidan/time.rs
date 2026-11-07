//! date & time

/// Keeps the clock in sync over the network (chronyd on Fedora), it catches up once online.
///
/// # Errors
/// `timedatectl` failed, e.g. no NTP service is installed.
pub async fn enable_ntp() -> crate::Res<()> {
    crate::root("timedatectl", ["set-ntp", "true"]).await
}
