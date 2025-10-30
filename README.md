# rsslowloris  
High-performance **HTTP slow-loris** written in **async Rust**.  
Keeps thousands of TCP connections half-open and dribbles useless headers to exhaust the target’s concurrent request pool.


---

## What it does  
1. Opens `N` TCP connections with `Connection: keep-alive`.  
2. Sends a valid HTTP request line + minimal headers.  
3. Every `--interval` seconds appends one random header and flushes.  
4. Server waits forever for the final `\r\n\r\n` → connection slot occupied.  
5. Once the server’s worker limit is reached, new legitimate clients get **503/timeout**.

---

## Technical limits  
| Resource | Practical ceiling¹ | Bottleneck |
|----------|-------------------|------------|
| **Open sockets per process** | ≈ 60 k on Linux | `ulimit -n` (raise with `ulimit -n 100000`) |
| **Memory per socket** | ± 1.2 kB (Tokio + kernel) | 100 k sockets ≈ 120 MB user-space |
| **CPU @ 100 k sockets** | 1–2 % of one core | epoll/kqueue is O(1) |
| **Packet rate** | 100 k ÷ 15 s = 6.7 k pps | Negligible on GbE |
| **Maximum throughput** | 0 (no body) | Only tiny TCP ACK + header |
| **OS tuning required** | YES | See **Tuning** section |

¹Measured on Ryzen 7 5800X, 64 GB RAM, Ubuntu 22.04, 100 Mbit/s link.

---

## Quick start  
```
# 1. raise fd limit for current shell
ulimit -n 100000

# 2. run against your own nginx container
docker run --rm -d --name nginx -p 8080:80 nginx:alpine
rust-slowloris -H 127.0.0.1 -p 8080 -s 50000 --interval 15
```

Watch the connection count in real time:
```
watch -n1 'ss -tn state established dst :8080 | wc -l'
```

---

## Installation  

### From source
```
git clone https://github.com/s-b-repo/rust-slowloris/
cd rust-slowloris
cargo build --release --features jemalloc
sudo cp target/release/rust-slowloris /usr/local/bin
```

---

## Tuning Linux for > 30 k sockets  
Add to `/etc/sysctl.conf` (root required):
```
# socket memory
net.ipv4.tcp_mem = 786432 1048576 26777216
net.ipv4.tcp_rmem = 4096 4096 16777216
net.ipv4.tcp_wmem = 4096 4096 16777216

# port exhaustion / TIME_WAIT reuse
net.ipv4.ip_local_port_range = 1024 65535
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 15

# connection tracking (if iptables)
net.netfilter.nf_conntrack_max = 2000000

# file descriptors
fs.file-max = 2000000
```
Apply: `sudo sysctl -p`

Raise user limits:
```
# /etc/security/limits.conf
* soft nofile 100000
* hard nofile 100000
```
Re-login or `sudo prlimit --nofile=100000:100000 --pid $$`.

---

## Flag reference  
```
-H, --host <HOST>        target IP or domain (required)
-p, --port <PORT>        target port [default: 80]
-s, --sockets <N>        concurrent TCP connections [default: 8000]
-i, --interval <SEC>     seconds between headers [default: 15]
-v, --verbose            print per-socket events
-h, --help               print help
```

---

## Exit codes  
| Code | Meaning |
|------|---------|
| 0 | Clean shutdown (Ctrl-C or duration reached) |
| 1 | CLI argument error |
| 2 | Operating-system limit (fd, memory) hit |

---

## Benchmarks  
| Sockets | RAM (RSS) | CPU (i7-12700H) | PPS out | OS |
|---------|-----------|-----------------|---------|----|
| 10 k | 14 MB | 0.3 % | 670 | Ubuntu 22.04 |
| 50 k | 68 MB | 0.9 % | 3.3 k | Ubuntu 22.04 |
| 100 k | 135 MB | 1.8 % | 6.7 k | Ubuntu 22.04 |

---

## Detecting the attack  
Server-side indicators:
* Sudden plateau at max workers (`nginx status module` → `Writing = limit`).  
* Thousands of ESTAB sockets in `ss -tn state established`.  
* No request body logged (CL 0).  
* Average CPU **low**, but new users time-out.

Mitigations:
* Lower `keepalive_timeout` (nginx) / `RequestReadTimeout` (Apache).  
* Drop slow clients with `mod_reqtimeout`, `ngx_http_limit_req`, or a WAF.  
* Use a reverse-proxy that terminates idle connections aggressively (Envoy, HAProxy, Cloudflare).

---

## Legal notice  
**Only run against hosts you own or have explicit written permission to test.**  
Unauthorized use is illegal in most jurisdictions and violates the GitHub Acceptable Use Policy.

---
