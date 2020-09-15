## Benchmarking 

### Setup Redis 5.0.3

```
$ cd ~/redis-5.0.8
$ echo never > /sys/kernel/mm/transparent_hugepage/enabled
With Linux Network stack:
$ ./src/redis-server
With VMA:
$ LD_PRELOAD=/root/libvma/src/vma/.libs/libvma.so ./src/redis-server
```

Enable remote access:
```
$ redis-cli
127.0.0.1:6379> CONFIG SET protected-mode no
```

## Setup Client

Use `redis-benchmark` tool.

* Key-space: 0..10000 (`-r`)
* Value-length: 8 bytes (`-d`)
* No. Requests: 1000000 (`-n`)

```cfg
With 1 connection:
$ redis-benchmark -p 6379 -t set,get -r 10000 -n 1000000 -e -d 8 -h 192.168.100.117 -c 1

For seastore (instead of redis):
Use port `-p 6666`, and you have to run twice once 

For VMA:
Add `LD_PRELOAD=/root/libvma/src/vma/.libs/libvma.so` in front.

With 50 connections:
Use `-c 50`.

For pipelineing:
Use `-P 29`. You may have to increase `-n` here.
```

### Results using redis

```
op,clients,tps,client_stack,server_stack,pipeline
set,1,33403.48,linux,linux,1
get,1,34702.94,linux,linux,1
set,1,50045.04,vma,linux,1
get,1,52579.00,vma,linux,1
set,1,116049.67,vma,vma,1
get,1,118008.02,vma,vma,1
set,50,119274.81,linux,linux,1
get,50,122144.86,linux,linux,1
set,50,381242.88,vma,vma,1
get,50,379794.94,vma,vma,1
set,50,1165501.25,linux,linux,29
get,50,1426533.62,linux,linux,29
set,50,1605136.38,vma,vma,29
get,50,2074258.38,vma,vma,29
```


## Setup sashstore

```
$ cd ~/sashstore
With Linux Network stack:
$ RUST_LOG=info cargo run --release -- --threads 1 --transport tcp
With VMA:
$ LD_PRELOAD=/root/libvma/src/vma/.libs/libvma.so RUST_LOG=info cargo run --release -- --threads 1 --transport tcp
```

Benchmark without network:
```
cargo bench --bin sashstore
```

Should yield:
```
test tests::bench_get_requests ... bench:         530 ns/iter (+/- 1)
test tests::bench_set_requests ... bench:         825 ns/iter (+/- 28)
```

### Results using sashstore

```cfg
op,client,tps,client_stack,server_stack,pipeline
set,1,33960.47,linux,linux,1
get,1,34261.83,linux,linux,1
set,1,54833.58,vma,linux,1
get,1,51522.49,vma,linux,1
set,50,116618.08,linux,linux,1
get,50,128155.84,linux,linux,1
```

# Tools installation instructions

redis-server:

```
wget http://download.redis.io/releases/redis-5.0.8.tar.gz
```

redis-benchmark tool:

```bash
apt install redis-tools
```

libvma:

```bash
git clone https://github.com/Mellanox/libvma.git
cd libvma
git checkout 8.7.0
./autogen.sh
./configure --with-ofed=/usr --prefix=/usr --libdir=/usr/lib64 --includedir=/usr/include --docdir=/usr/share/doc/libvma --sysconfdir=/etc
make -j12
sudo make install
sudo /etc/init.d/vma start
ulimit -l unlimited
```