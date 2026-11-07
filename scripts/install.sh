DESTDIR=${DESTDIR:-/}

mkdir -p $DESTDIR/usr/share/taidan
cp -r data/tweaks/ $DESTDIR/usr/share/taidan/
cp -r data/avatars/ $DESTDIR/usr/share/taidan/
# read by KWin before the OOBE starts, see taidan-initial-setup.service
cp -r data/session-config/ $DESTDIR/usr/share/taidan/
# translations, read at runtime in release builds (src/l10n.rs)
cp -r po/ $DESTDIR/usr/share/taidan/

install -Dpm644 data/sysusers.d/taidan.conf             -t $DESTDIR/usr/lib/sysusers.d/
install -Dpm644 data/systemd/*.service                  -t $DESTDIR/usr/lib/systemd/system/
install -Dpm644 data/systemd/*.preset                   -t $DESTDIR/usr/lib/systemd/system-preset/
# copied to /run/polkit-1/rules.d only while the OOBE runs, see taidan-initial-setup.service
install -Dpm644 data/polkit-1/rules.d/100-taidan.rules  -t $DESTDIR/usr/share/taidan/polkit/
install -Dpm644 data/pam.d/taidan                       -t $DESTDIR/etc/pam.d/
# the taidan user's home
install -dm755 $DESTDIR/usr/lib/taidan
install -Dpm755 data/start-taidan                       -t $DESTDIR/usr/libexec/
