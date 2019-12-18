echo + target remote localhost:1234\n
target remote localhost:1234

echo + symbol-file kernel\n
symbol-file build/kernel.bin
add-symbol-file sys/init/build/init 0x228000
