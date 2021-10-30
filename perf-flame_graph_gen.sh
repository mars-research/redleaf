#!/bin/bash

python3 perf_parser.py

./flamegraph.pl out.kern_folded > perf.svg