use parking_lot::Mutex;
use smoltcp::{
    iface::{SocketHandle as InnerSocketHandle, SocketSet},
    socket::{tcp, udp},
    time::Duration,
};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// `BufferSize` is used to configure the size of the socket buffer.
#[derive(Debug, Clone, Copy)]
pub struct BufferSize {
    pub tcp_rx_size: usize,
    pub tcp_tx_size: usize,

    pub udp_rx_size: usize,
    pub udp_tx_size: usize,
    pub udp_rx_meta_size: usize,
    pub udp_tx_meta_size: usize,
}

impl BufferSize {
    /// Conservative preset: Low memory usage (~32KB per connection)
    /// Suitable for: Memory-constrained systems, many connections
    /// Performance: ~256Mbps at 1ms RTT, ~25Mbps at 10ms RTT
    pub fn conservative() -> Self {
        BufferSize {
            tcp_rx_size: 16 * 1024,   // 16KB
            tcp_tx_size: 16 * 1024,   // 16KB
            udp_rx_size: 8192,
            udp_tx_size: 8192,
            udp_rx_meta_size: 32,
            udp_tx_meta_size: 32,
        }
    }

    /// Moderate preset: Balanced (~128KB per connection)
    /// Suitable for: General use, moderate number of connections
    /// Performance: ~1Gbps at 1ms RTT, ~100Mbps at 10ms RTT
    pub fn moderate() -> Self {
        BufferSize {
            tcp_rx_size: 64 * 1024,   // 64KB
            tcp_tx_size: 64 * 1024,   // 64KB
            udp_rx_size: 8192,
            udp_tx_size: 8192,
            udp_rx_meta_size: 32,
            udp_tx_meta_size: 32,
        }
    }

    /// Aggressive preset: High performance (~1MB per connection)
    /// Suitable for: High-bandwidth scenarios, few connections, ample memory
    /// Performance: ~4Gbps at 1ms RTT, ~400Mbps at 10ms RTT
    pub fn aggressive() -> Self {
        BufferSize {
            tcp_rx_size: 512 * 1024,  // 512KB
            tcp_tx_size: 512 * 1024,  // 512KB
            udp_rx_size: 8192,
            udp_tx_size: 8192,
            udp_rx_meta_size: 32,
            udp_tx_meta_size: 32,
        }
    }

    /// Get buffer size from environment variable or use default
    /// EASYTIER_TCP_BUFFER_PROFILE: conservative, moderate, aggressive
    /// EASYTIER_TCP_BUFFER_SIZE: custom size in KB
    pub fn from_env() -> Self {
        // Check for profile setting
        if let Ok(profile) = std::env::var("EASYTIER_TCP_BUFFER_PROFILE") {
            match profile.to_lowercase().as_str() {
                "conservative" => return Self::conservative(),
                "moderate" => return Self::moderate(),
                "aggressive" => return Self::aggressive(),
                _ => tracing::warn!("Unknown buffer profile '{}', using default", profile),
            }
        }

        // Check for custom size setting
        if let Ok(size_kb) = std::env::var("EASYTIER_TCP_BUFFER_SIZE") {
            if let Ok(kb) = size_kb.parse::<usize>() {
                let bytes = kb * 1024;
                tracing::info!("Using custom TCP buffer size: {}KB", kb);
                return BufferSize {
                    tcp_rx_size: bytes,
                    tcp_tx_size: bytes,
                    udp_rx_size: 8192,
                    udp_tx_size: 8192,
                    udp_rx_meta_size: 32,
                    udp_tx_meta_size: 32,
                };
            }
        }

        // Default to moderate
        Self::moderate()
    }
}

impl Default for BufferSize {
    fn default() -> Self {
        // Use moderate profile by default as a balance between performance and memory
        // This provides good performance (~1Gbps at 1ms RTT) while limiting memory usage
        // to ~128KB per connection (64KB RX + 64KB TX)
        //
        // Memory usage estimates:
        // - 100 connections: ~12.5MB
        // - 1000 connections: ~125MB
        // - 10000 connections: ~1.25GB
        //
        // For different scenarios, use environment variables:
        // - EASYTIER_TCP_BUFFER_PROFILE=conservative (16KB, low memory)
        // - EASYTIER_TCP_BUFFER_PROFILE=moderate (64KB, balanced - default)
        // - EASYTIER_TCP_BUFFER_PROFILE=aggressive (512KB, high performance)
        // - EASYTIER_TCP_BUFFER_SIZE=128 (custom size in KB)
        Self::from_env()
    }
}

type SharedSocketSet = Arc<Mutex<SocketSet<'static>>>;

#[derive(Clone)]
pub struct SocketAlloctor {
    sockets: SharedSocketSet,
    buffer_size: BufferSize,
}

impl SocketAlloctor {
    pub(crate) fn new(buffer_size: BufferSize) -> SocketAlloctor {
        let sockets = Arc::new(Mutex::new(SocketSet::new(Vec::new())));
        SocketAlloctor {
            sockets,
            buffer_size,
        }
    }
    pub(crate) fn sockets(&self) -> &SharedSocketSet {
        &self.sockets
    }
    pub fn new_tcp_socket(&self) -> SocketHandle {
        let mut set = self.sockets.lock();
        let handle = set.add(self.alloc_tcp_socket());
        SocketHandle::new(handle, self.sockets.clone())
    }
    fn alloc_tcp_socket(&self) -> tcp::Socket<'static> {
        let rx_buffer = tcp::SocketBuffer::new(vec![0; self.buffer_size.tcp_rx_size]);
        let tx_buffer = tcp::SocketBuffer::new(vec![0; self.buffer_size.tcp_tx_size]);
        let mut tcp = tcp::Socket::new(rx_buffer, tx_buffer);
        tcp.set_nagle_enabled(false);
        tcp.set_keep_alive(Some(Duration::from_secs(10)));
        tcp.set_timeout(Some(Duration::from_secs(60)));

        tcp
    }

    pub fn new_udp_socket(&self) -> SocketHandle {
        let mut set = self.sockets.lock();
        let handle = set.add(self.alloc_udp_socket());
        SocketHandle::new(handle, self.sockets.clone())
    }

    fn alloc_udp_socket(&self) -> udp::Socket<'static> {
        let rx_buffer = udp::PacketBuffer::new(
            vec![udp::PacketMetadata::EMPTY; self.buffer_size.udp_rx_meta_size],
            vec![0; self.buffer_size.udp_rx_size],
        );
        let tx_buffer = udp::PacketBuffer::new(
            vec![udp::PacketMetadata::EMPTY; self.buffer_size.udp_tx_meta_size],
            vec![0; self.buffer_size.udp_tx_size],
        );
        let udp = udp::Socket::new(rx_buffer, tx_buffer);

        udp
    }
}

pub struct SocketHandle(InnerSocketHandle, SharedSocketSet);

impl SocketHandle {
    fn new(inner: InnerSocketHandle, set: SharedSocketSet) -> SocketHandle {
        SocketHandle(inner, set)
    }
}

impl Drop for SocketHandle {
    fn drop(&mut self) {
        let mut iface = self.1.lock();
        iface.remove(self.0);
    }
}

impl Deref for SocketHandle {
    type Target = InnerSocketHandle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SocketHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
