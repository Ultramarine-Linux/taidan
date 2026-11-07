use crate::prelude::*;

/// # Errors
/// - if `flatpak` doesn't work correctly then maybe
pub(super) async fn handle_flatpak<C: crate::Callback>(
    _: &C,
    f: impl FnOnce(&mut tokio::process::Command) -> &mut tokio::process::Command,
) -> Res<()> {
    super::dnf::run_logged("flatpak", f, |_| {}).await
}
