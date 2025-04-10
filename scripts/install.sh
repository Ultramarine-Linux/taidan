DESTDIR=${DESTDIR:-/}
appid="com.fyralabs.Taidan"

for category in catalogue/*; do
  install -Dpm644 $category -t $DESTDIR/etc/$appid/catalogue/
done

install -Dpm644 data/sysusers.d/taidan.conf             -t $DESTDIR/usr/lib/sysusers.d/
install -Dpm644 data/systemd/*.service                  -t $DESTDIR/usr/lib/systemd/system/
install -Dpm644 data/systemd/*.preset                   -t $DESTDIR/usr/lib/systemd/system-preset/
install -Dpm644 data/polkit-1/rules.d/100-taidan.rules  -t $DESTDIR/usr/share/polkit-1/rules.d/
install -Dpm644 data/pam.d/taidan -t $DESTDIR/etc/pam.d/
install -Dpm644 data/labwc/*                               -t $DESTDIR/usr/lib/taidan/labwc/