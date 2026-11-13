DESTDIR=${DESTDIR:-/}

taidan_data=$DESTDIR/usr/share/taidan

mkdir -p $taidan_data
cp -r data/tweaks/ data/avatars/ data/session-config/ po/ $taidan_data/

install -Dpm644 data/sysusers.d/taidan.conf             -t $DESTDIR/usr/lib/sysusers.d/
install -Dpm644 data/systemd/*.service                  -t $DESTDIR/usr/lib/systemd/system/
install -Dpm644 data/systemd/*.preset                   -t $DESTDIR/usr/lib/systemd/system-preset/
install -Dpm644 data/polkit-1/rules.d/100-taidan.rules  -t $taidan_data/polkit/
install -Dpm644 data/pam.d/taidan                       -t $DESTDIR/etc/pam.d/
install -dm755 $DESTDIR/usr/lib/taidan
install -Dpm755 data/start-taidan                       -t $DESTDIR/usr/libexec/
