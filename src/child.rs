use std::process::Child;
use std::time::{Duration, Instant};

use parking_lot::{Mutex, MutexGuard};

const STOP_GRACE: Duration = Duration::from_secs(2);
const STOP_POLL: Duration = Duration::from_millis(50);

pub struct ChildSlot<Tag = ()> {
    name: &'static str,
    slot: Mutex<Option<(Tag, Child)>>,
}

impl<Tag> ChildSlot<Tag> {
    pub const fn new(name: &'static str) -> Self {
        Self { name, slot: Mutex::new(None) }
    }

    pub fn lock(&self) -> MutexGuard<'_, Option<(Tag, Child)>> {
        self.slot.lock()
    }

    pub fn replace(&self, tag: Tag, child: Option<Child>) -> Option<Child> {
        std::mem::replace(&mut *self.lock(), child.map(|c| (tag, c))).map(|(_, c)| c)
    }

    pub fn take(&self) -> Option<Child> {
        self.lock().take().map(|(_, c)| c)
    }

    pub fn stop(&self) {
        if let Some(child) = self.take() {
            terminate(self.name, child);
        }
    }
}

pub fn terminate(name: &str, mut child: Child) {
    if let Ok(pid) = libc::pid_t::try_from(child.id())
        // SAFETY: the child hasn't been waited for, so the pid is still its own and not reused
        && unsafe { libc::kill(pid, libc::SIGTERM) } != 0
    {
        tracing::warn!(e = ?std::io::Error::last_os_error(), name, "cannot stop the process");
    }
    let deadline = Instant::now() + STOP_GRACE;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(STOP_POLL),
            Ok(None) => break,
            Err(e) => {
                tracing::warn!(?e, name, "cannot wait for the process");
                break;
            }
        }
    }
    tracing::warn!(name, "the process didn't exit, killing it");
    if let Err(e) = child.kill() {
        tracing::warn!(?e, name, "cannot kill the process");
    }
    if let Err(e) = child.wait() {
        tracing::warn!(?e, name, "cannot wait for the process");
    }
}
