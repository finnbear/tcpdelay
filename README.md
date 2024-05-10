# tcpdelay

A TCP proxy that simulates latency and jitter.

## Deprecation
The following is much better (more flexibility, supports UDP, packet-wise):
```console
# one-way latency, jitter, correlation, loss, corruption
sudo tc qdisc add dev lo root netem delay 50ms 10ms 25% loss 5% corrupt 1%

# undo
sudo tc qdisc delete dev lo root
```

## Installation

```console
cargo install --git https://github.com/finnbear/tcpdelay
```

## Usage

```console
tcpdelay 0.1.0
Simulates latency on proxied TCP connections

USAGE:
    tcpdelay [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -q, --quiet      Don't log anything
    -V, --version    Prints version information

OPTIONS:
    -d, --downstream <downstream>    Downstream TCP domain/ip/port (to forward connections to) [default: 127.0.0.1:8080]
    -j, --jitter <jitter>            Max additional one-way latency (millis) [default: 25]
    -l, --latency <latency>          Base one-way latency (millis) [default: 75]
    -u, --upstream <upstream>        Upstream TCP port on localhost (to forward connections from) [default: 8081]
```
