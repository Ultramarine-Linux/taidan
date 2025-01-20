DESTDIR=${DESTDIR:-/}
schemadir="/usr/share/taidan/window-manager/glib-2.0/schemas"
mkdir -p $DESTDIR$schemadir
# GSettings insists on the override files being in the same directory as the
# schemas they modify, so pretend that this is the case with symlinks and
# create the compiled schema.
ln -s -f /usr/share/glib-2.0/schemas/org.gnome.desktop.wm.keybindings.gschema.xml $DESTDIR$schemadir
ln -s -f /usr/share/glib-2.0/schemas/org.gnome.desktop.wm.preferences.gschema.xml $DESTDIR$schemadir
ln -s -f /usr/share/glib-2.0/schemas/org.gnome.desktop.enums.xml                  $DESTDIR$schemadir
install -Dpm644 data/window-manager/*.override                                 -t $DESTDIR$schemadir
glib-compile-schemas --strict $DESTDIR$schemadir
