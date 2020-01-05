echo + target remote localhost:1234\n
target remote localhost:1234

echo + symbol-file kernel\n
symbol-file build/kernel.bin
#add-symbol-file sys/init/build/init 0x228000
lay sp
add-symbol-file usr/xv6/kernel/fs/build/xv6fs 0xaea7050
#add-symbol-file sys/dev/ahci/build/ahci 0xd90050
b build_domain_fs
b fs/lib.rs:109

