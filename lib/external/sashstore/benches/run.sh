#!/bin/bash
set -ex
cargo build --release

HOSTNAME=`hostname`
MAX_CORES=`nproc`

rm *.log *.csv || true

CAPACITY=10000000
RUNTIME=10

declare -a benchmarks=("andreamap" "std" "index" "indexmap")
declare -a distributions=("uniform" "skewed")
declare -a write_ratios=("10" "20")

# Maybe: disable dvfs
# echo performance | tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
LOGFILE=${HOSTNAME}_results.log
CSVFILE=${HOSTNAME}_results.csv

for benchmark in "${benchmarks[@]}"; do
    for distribution in "${distributions[@]}"; do
        for write_ratio in "${write_ratios[@]}"; do

            if [ ! -f "$CSVFILE" ]; then
                echo "benchmark,threads,write_ratio,capacity,dist,tid,total_ops,heap_total,duration" | tee $CSVFILE
            fi

            for cores in `seq 0 4 $MAX_CORES`; do
                if [ "$cores" -eq "0" ]; then
                    cores=1
                fi
                (cargo bench --bench hashbench -- -b ${benchmark} --capacity $CAPACITY --runtime $RUNTIME --threads $cores --write-ratio $write_ratio --distribution $distribution | tee -a $CSVFILE) 3>&1 1>&2 2>&3 | tee -a $LOGFILE
            done
        done
    done
done

python3 plot.py $CSVFILE