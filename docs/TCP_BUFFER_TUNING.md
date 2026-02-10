# TCP Buffer Size Configuration Guide

## Overview

EasyTier uses adaptive TCP buffer sizing to balance performance and memory usage. This document explains how TCP buffers work and how to configure them for your use case.

## Understanding TCP Buffers

TCP buffer sizes directly impact network performance through the **Bandwidth-Delay Product (BDP)**:

```
Maximum Throughput = Buffer Size / Round-Trip Time (RTT)
```

### Example Calculations

| Buffer Size | 1ms RTT | 10ms RTT | 50ms RTT |
|------------|---------|----------|----------|
| 16KB | 128 Mbps | 12.8 Mbps | 2.56 Mbps |
| 64KB | 512 Mbps | 51.2 Mbps | 10.24 Mbps |
| 256KB | 2 Gbps | 204.8 Mbps | 40.96 Mbps |
| 512KB | 4 Gbps | 409.6 Mbps | 81.92 Mbps |

## Memory Usage Considerations

Each TCP connection uses buffer memory for both receive (RX) and transmit (TX):

| Profile | Per Connection | 100 Connections | 1000 Connections | 10000 Connections |
|---------|---------------|-----------------|------------------|-------------------|
| Conservative (16KB × 2) | 32 KB | 3.2 MB | 32 MB | 320 MB |
| Moderate (64KB × 2) | 128 KB | 12.5 MB | 125 MB | 1.25 GB |
| Aggressive (512KB × 2) | 1 MB | 100 MB | 1 GB | 10 GB |

## Buffer Size Profiles

EasyTier provides three predefined profiles:

### 1. Conservative Profile
```bash
EASYTIER_TCP_BUFFER_PROFILE=conservative
```
- **Buffer Size**: 16KB RX + 16KB TX = 32KB per connection
- **Performance**: ~256 Mbps at 1ms RTT, ~25 Mbps at 10ms RTT
- **Memory**: 32KB per connection
- **Best For**: 
  - Memory-constrained systems (IoT, embedded devices)
  - Many concurrent connections (1000+)
  - Lower bandwidth requirements

### 2. Moderate Profile (Default)
```bash
EASYTIER_TCP_BUFFER_PROFILE=moderate  # or not set
```
- **Buffer Size**: 64KB RX + 64KB TX = 128KB per connection
- **Performance**: ~1 Gbps at 1ms RTT, ~100 Mbps at 10ms RTT
- **Memory**: 128KB per connection
- **Best For**:
  - General purpose deployments
  - Balanced performance and memory usage
  - Most typical use cases

### 3. Aggressive Profile
```bash
EASYTIER_TCP_BUFFER_PROFILE=aggressive
```
- **Buffer Size**: 512KB RX + 512KB TX = 1MB per connection
- **Performance**: ~4 Gbps at 1ms RTT, ~400 Mbps at 10ms RTT
- **Memory**: 1MB per connection
- **Best For**:
  - High-bandwidth scenarios
  - Few concurrent connections (< 100)
  - Systems with ample memory (8GB+)
  - Data center or cloud deployments

## Custom Buffer Sizes

For fine-grained control, set a custom buffer size in kilobytes:

```bash
EASYTIER_TCP_BUFFER_SIZE=128  # 128KB buffers (256KB total per connection)
```

This sets both RX and TX buffers to the specified size.

## Configuration Examples

### Example 1: Home Router (Limited Memory)
```bash
# Conservative profile for home router with 512MB RAM
export EASYTIER_TCP_BUFFER_PROFILE=conservative
easytier-core --network-name mynet --network-secret mysecret
```

### Example 2: Cloud Server (High Performance)
```bash
# Aggressive profile for cloud server with 16GB RAM
export EASYTIER_TCP_BUFFER_PROFILE=aggressive
easytier-core --network-name mynet --network-secret mysecret
```

### Example 3: Custom Tuning
```bash
# Custom 128KB buffers (256KB total per connection)
export EASYTIER_TCP_BUFFER_SIZE=128
easytier-core --network-name mynet --network-secret mysecret
```

### Example 4: Docker Container
```yaml
# docker-compose.yml
version: '3'
services:
  easytier:
    image: easytier/easytier
    environment:
      - EASYTIER_TCP_BUFFER_PROFILE=moderate
    command: --network-name mynet --network-secret mysecret
```

## Comparison with Other Systems

### Linux Kernel TCP Buffers
Linux uses adaptive buffer sizing with defaults:
```
tcp_rmem: min=4KB, default=128KB, max=6MB
tcp_wmem: min=4KB, default=16KB, max=4MB
```

EasyTier's moderate profile (64KB) is comparable to Linux's defaults.

### Other VPN Solutions
- **WireGuard**: Uses fixed kernel buffers, relies on Linux TCP stack
- **ZeroTier**: Uses flow control with smaller fixed buffers
- **Tailscale**: Built on WireGuard, inherits its buffer strategy

## Monitoring and Troubleshooting

### Signs You Need Larger Buffers
- TCP throughput significantly lower than UDP
- High latency but low packet loss
- Network utilization below capacity
- `iperf3` TCP tests show poor performance

### Signs You Need Smaller Buffers
- High memory usage
- Out-of-memory errors
- System becomes unresponsive with many connections
- Memory pressure affecting other services

### Testing Performance
Use `iperf3` to test TCP performance:

```bash
# Server side
iperf3 -s

# Client side (test TCP)
iperf3 -c <server-ip> -t 30

# Client side (test UDP for comparison)
iperf3 -c <server-ip> -u -b 100M -t 30
```

## Recommendations by Use Case

| Use Case | Recommended Profile | Expected Connections | Notes |
|----------|-------------------|---------------------|-------|
| Home Network | Conservative | 10-50 | Limited memory, moderate bandwidth |
| Small Office | Moderate | 50-200 | Balanced usage |
| Data Center | Aggressive | 10-100 | High bandwidth, low latency |
| IoT Gateway | Conservative | 100-1000 | Very limited memory |
| Cloud VPN | Aggressive | 10-50 | High bandwidth requirements |
| Mobile Hotspot | Conservative | 5-20 | Limited memory and bandwidth |

## Advanced Topics

### Why Not Always Use Large Buffers?

While larger buffers enable higher throughput, they have drawbacks:

1. **Memory Exhaustion**: With many connections, large buffers can exhaust available RAM
2. **Bufferbloat**: Large buffers can increase latency under network congestion
3. **Waste**: Idle connections hold allocated memory
4. **Cache Pressure**: Large allocations can reduce CPU cache effectiveness

### Future Improvements

Potential enhancements being considered:
- Dynamic buffer sizing based on actual connection throughput
- Per-connection buffer adjustment
- Global memory limits with buffer pool management
- Integration with OS TCP buffer autotuning

## References

- [Linux TCP Buffer Tuning](https://www.kernel.org/doc/Documentation/networking/ip-sysctl.txt)
- [Bandwidth-Delay Product](https://en.wikipedia.org/wiki/Bandwidth-delay_product)
- [Understanding TCP Buffer Sizes](https://blog.cloudflare.com/the-story-of-one-latency-spike/)
