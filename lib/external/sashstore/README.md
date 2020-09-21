# sashstore

Aims to be a simple (but safe) partitioned key--value store in rust.

## main binary

Run with:

```bash
cargo run --release -- --threads 1 --transport tcp
```

To benchmark use `redis-benchmark`:

```bash
redis-benchmark -t get -r 10000 -n 1000000 -e -d 8 -h 192.168.100.117 -p 6666
```

## hashbench

Benchmarks partitioned hash-table implementations:

```bash
cd benches
bash run.sh
```

## Application benchmarks

```bash
$ cargo bench --bin sashstore
test tests::bench_get_requests ... bench:         414 ns/iter (+/- 6)
test tests::bench_set_requests ... bench:         613 ns/iter (+/- 19)
```

## sashstore

Server

```bash
RUST_LOG=trace cargo run
```

Client (send one set request):

```bash
./clients/memslap --udp --test=set --execute-number=1 --server=127.0.0.1:6666
```

Benchmark client (send many requests):

```bash
./clients/memaslap -s 127.0.0.1:6666 -U -S 1s -T1 -c 1
```

Investigate with tcpdump:

```bash
sudo tcpdump -i lo udp port 6666 -vv -X
```

## memcached

```bash
sudo apt-get install memcached
sudo apt-get build-dep libmemcached
wget https://launchpad.net/libmemcached/1.0/1.0.18/+download/libmemcached-1.0.18.tar.gz
tar zxvf libmemcached-1.0.18.tar.gz
cd libmemcached-1.0.18
./configure --enable-memaslap
make -j 6
sudo make install
```

Start server (UDP)

```bash
/usr/bin/memcached -m 64 -U 11211 -u memcache -l 127.0.0.1
```

Start client

```bash
./clients/memaslap -s 127.0.0.1:11211 -U -S 1s -T1 -c 1
```

```bash
sudo tcpdump -i lo udp port 11211 -vv -X
```

```log
Get Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        309339       309339       317.8      259976     85       810        214        67.27      204.98
Global   11       3369183      306289       321.0      1895615    58       1054       216        62.39      208.72

Set Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        34375        34375        35.3       0          87       696        216        69.96      206.26
Global   11       374420       34038        35.7       0          56       738        218        64.62      210.00

Total Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        343714       343714       353.1      259981     85       810        214        68.18      205.12
Global   11       3743617      340328       356.6      1895626    56       1054       216        63.31      208.85
```
