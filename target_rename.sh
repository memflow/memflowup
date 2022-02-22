#!/bin/sh

arch=$(case "$1" in
	x86_64-*) echo "x86_64";;
	i686-*|i586-*) echo "x86";;
	arm-*|armv7-*) echo "arm";;
	aarch64-*) echo "aarch64";;
esac)

ext=$(case "$1" in
	*-linux-*) echo ".so";;
	*-windows-*) echo ".dll";;
	*-apple-*) echo ".dylib";;
esac)

for f in ./target/$1/release/*$ext; do
	if [ -f $f ] && [ -x $f ]; then
		name=${f%.*}
		farch="${name##*.}"
		if [ "$farch" != "$arch" ]; then
			newloc="$name.$arch$ext"
			mv "$f" "$newloc"
			echo "$newloc"
		fi
	fi
done
