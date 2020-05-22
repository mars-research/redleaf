#!/bin/bash

# TODO: Pick this up from lscpu or derive it from msrs.
CPU_FREQ=2600000

set_freq() {
	# make both min and max to the advertised freq
	if [ -d /sys/devices/system/cpu/cpu0/cpufreq/ ]; then
		for i in $(ls /sys/devices/system/cpu/cpu*/cpufreq/scaling_max_freq); do echo "${CPU_FREQ}" | sudo tee $i > /dev/null 2>&1 ;done
		for i in $(ls /sys/devices/system/cpu/cpu*/cpufreq/scaling_min_freq); do echo "${CPU_FREQ}" | sudo tee $i > /dev/null 2>&1 ;done
	fi
}

disable_cstate() {
	echo "Disabling C-states"
	for i in $(ls /sys/devices/system/cpu/cpu*/cpuidle/state*/disable); do echo "1" | sudo tee $i > /dev/null 2>&1 ;done
}

disable_turbo() {
	if ! [ -x "$(command -v rdmsr)" ]; then
		echo "Installing msr-tools ..."
		sudo apt install msr-tools
	fi

	# make sure we have this module loaded
	if [ -z "$(lsmod | grep msr)" ]; then
		echo "Loading msr module"
		sudo modprobe msr
	fi

	# disable turbo boost (bit 38 on 0x1a0 msr)
	echo "Disabling turboboost"
	sudo wrmsr -a 0x1a0 $(printf "0x%x" $(($(sudo rdmsr -d 0x1a0)|(1<<38))))
}

set_const_freq() {
	set_freq;

	disable_cstate;

	disable_turbo;
}

dump_sys_state() {
	if [ -d /sys/devices/system/cpu/cpu0/cpufreq/ ]; then
		for i in $(ls /sys/devices/system/cpu/cpu*/cpufreq/scaling_max_freq); do echo "$i: $(cat $i)";done
		for i in $(ls /sys/devices/system/cpu/cpu*/cpufreq/scaling_min_freq); do echo "$i: $(cat $i)";done
	fi

	for i in $(ls /sys/devices/system/cpu/cpu*/cpuidle/state*/disable); do echo "$i: $(cat $i)";done
	sudo rdmsr -a 0x1a0 -f 38:38
}

set_const_freq;
dump_sys_state;
