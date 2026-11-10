//! date & time

/// turns on network time sync (chronyd on Fedora), which catches up once online
///
/// # Errors
/// `timedatectl` failed, e.g. no NTP service is installed
pub async fn enable_ntp() -> crate::Res<()> {
    crate::root(crate::Verb::EnableNtp).await
}
