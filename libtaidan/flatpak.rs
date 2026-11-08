use crate::prelude::*;

/// # Errors
/// - if `flatpak` doesn't work correctly then maybe
pub(super) async fn handle_flatpak<C: crate::Callback>(_: &C, verb: crate::Verb) -> Res<()> {
    super::dnf::run_logged("flatpak", &verb, |_| {}).await
}
