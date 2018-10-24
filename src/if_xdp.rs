// XDP Rust bindings for if_xdp.h

// XDP socket options from if_xdp.h
pub static XDP_MMAP_OFFSETS: usize =  1;
pub static XDP_RX_RING: usize = 2;
pub static XDP_TX_RING: usize = 3;
pub static XDP_UMEM_REG: usize = 4;
pub static XDP_UMEM_FILL_RING: usize = 5;
pub static XDP_UMEM_COMPLETION_RING: usize = 6;
pub static XDP_STATISTICS: usize = 7;

// XDP mmap page offsets
pub static XDP_PGOFF_RX_RING: u64 = 0;
pub static XDP_PGOFF_TX_RING: u64 = 0x80000000;
pub static XDP_UMEM_PGOFF_FILL_RING: u64 = 0x100000000;
pub static XDP_UMEM_PGOFF_COMPLETION_RING: u64 = 0x180000000;

#[repr(C)]
#[derive(Default)]
pub struct XdpRingOffset {
    pub producer: u64,
    pub consumer: u64,
    pub desc: u64,
}

#[repr(C)]
#[derive(Default)]
pub struct XdpMmapOffsets {
    pub rx: XdpRingOffset,
    pub tx: XdpRingOffset,
    pub fr: XdpRingOffset, // fill
    pub cr: XdpRingOffset, // completion
}

#[repr(C)]
pub struct XdpUmemReg {
    pub addr: u64,
    pub len: u64,
    pub chunk_size: u32,
    pub headroom: u32,
}
