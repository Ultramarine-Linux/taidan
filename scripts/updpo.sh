xtr src/main.rs -o po/taidan.pot --package-name Taidan --package-version `sed -nE 's@^version = "(.+)"@\1@ p' Cargo.toml`
for file in po/*.po; do
    echo " ==> Merging $file"
    msgmerge -vU $file po/taidan.pot
    echo
done
rm po/*.po~
