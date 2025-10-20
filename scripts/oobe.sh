# This script is executed when the user confirm the setup


langlist=`dnf search ultramarine-langpacks | sed -n '/-core-/!p' | sed -n '/-fonts-/!p' | sed -E 's/\.noarch:.+//;s/ ultramarine-langpacks-//'` &

systemctl preset-all &

lang=`. /etc/locale.conf; echo ${LANG%.*}`
wait
if echo $langlist | grep $lang; then
  # stdout are packages that taidan will install
  # echo "pkg: ultramarine-langpacks-$lang"
elif echo $langlist | grep ${lang%_*}; then
  # echo "pkg: ultramarine-langpacks-${lang%_*}"
fi


# add video group
if grep -E '^video:' /etc/group; then
  # $1 is path to json file for `settings`
  usermod -aG video `jq -r '.username' $1`
fi

hostnamectl hostname ultramarine
