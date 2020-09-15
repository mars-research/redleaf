# Benchmarking memcached protocol

## sashstore

Running:

`./clients/memaslap -s 127.0.0.1:6666 -U -S 1s -T1 -c 1`

Using `--capacity 5000000`, compile settings LTO and codegen-units=1, jemalloc allocator:

```log
Get Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        50615        50615        61.8       0          15       185        17         3.12       17.11
Global   8        476657       59582        52.5       0          7        1299       14         5.92       14.10

Set Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        5623         5623         6.9        0          17       565        19         10.53      19.60
Global   8        52962        6620         5.8        0          8        713        17         6.08       16.60

Total Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        56237        56237        68.7       0          15       565        17         5.20       17.35
Global   8        529621       66202        58.4       0          7        1299       14         6.67       14.33
```

More clients: `./clients/memaslap -s 127.0.0.1:6666 -U -S 1s -T1 -c 30`

```log
Get Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        171732       171732       182.8      0          65       413        156        24.65      156.06
Global   8        1409116      176139       178.2      0          50       716        152        24.27      152.10

Set Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        19083        19083        20.3       0          67       418        157        24.36      156.82
Global   8        156584       19573        19.8       0          67       708        154        19.88      153.34

Total Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        190817       190817       203.1      0          65       418        157        18.02      156.14
Global   8        1565706      195713       198.0      0          50       716        153        18.05      152.23
```

## memcached

Running:

`./clients/memaslap -s 127.0.0.1:11211 -U -S 1s -T1 -c 1`

```log
Get Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        70442        70442        67.5       0          7        695        12         4.77       12.03
Global   6        390208       65034        73.1       0          7        1191       13         4.89       13.06

Set Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        7827         7827         7.5        0          8        350        13         10.02      13.06
Global   6        43357        7226         8.1        0          8        421        14         10.78      14.08

Total Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        78268        78268        75.0       0          7        695        12         5.74       12.13
Global   6        433567       72261        81.2       0          7        1191       13         5.99       13.16
```

More clients: `./clients/memaslap -s 127.0.0.1:6666 -U -S 1s -T1 -c 30`

```log
Get Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        152259       152259       157.1      73519      103      527        176        29.21      175.94
Global   7        1060143      151449       158.0      177530     29       598        177        32.48      176.40

Set Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        16918        16918        17.5       0          123      450        178        27.00      176.87
Global   7        117811       16830        17.6       0          62       610        179        29.87      177.31

Total Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        169175       169175       174.6      73515      103      527        177        23.64      176.03
Global   7        1177957      168279       175.5      177530     29       610        178        27.48      176.49
```
