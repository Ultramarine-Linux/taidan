for file in src/pages/*; do
    x=`echo $file | sed -E 's@.+/_(..)_.+@\1@'`
    x=${x#0}
    x=$((x))
    if [ $x -gt 7 ]; then
        x=$((x+1));
        if [ $x -le 9 ]; then
            x="0$x"
        fi
        mv $file `echo $file | sed -E "s@(.+)/_(..)_(.+)@\1/_${x}_\3@"`
    fi
done
