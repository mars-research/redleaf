echo + target remote localhost:1234\n
target remote localhost:1234

echo + symbol-file kernel\n
symbol-file build/redleaf.mb2
#add-symbol-file sys/init/build/init 0x228000

define btall
	thread apply all backtrace
end

source vermilion.py