# make both min and max to the advertised freq
for i in $(ls /sys/devices/system/cpu/cpu*/cpufreq/scaling_max_freq); do echo "2600000" | sudo tee $i;done
for i in $(ls /sys/devices/system/cpu/cpu*/cpufreq/scaling_min_freq); do echo "2600000" | sudo tee $i;done
# disable C states
for i in $(ls /sys/devices/system/cpu/cpu*/cpuidle/state*/disable); do echo "1" | sudo tee $i;done

