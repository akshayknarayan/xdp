// XDP Rust bindings for if_xdp.h
include!(concat!(env!("OUT_DIR"), "/if_xdp.rs"));

use libc::{c_int, c_void};

// values from https://github.com/torvalds/linux/blob/master/samples/bpf/xdpsock_user.c#L36
pub static SOL_XDP: c_int = 283;
pub static PF_XDP: c_int = 44;

pub static NUM_FRAMES: usize = 131072;
pub static FRAME_HEADROOM: usize = 0;
pub static FRAME_SHIFT: c_int = 11;
pub static FRAME_SIZE: usize = 2048;
pub static NUM_DESCS: c_int = 1024;
pub static BATCH_SIZE: usize = 16;

pub static FQ_NUM_DESCS: usize = 1024;
pub static CQ_NUM_DESCS: usize = 1024;

pub struct XdpUmemUqueue {
    pub cached_producer: u32,
    pub cached_consumer: u32,
    pub mask: u32,
    pub size: u32,
    pub producer: *mut u32,
    pub consumer: *mut u32,
    pub ring: *mut xdp_desc,
    pub map: *mut c_void,
}

pub struct UmemReadRingSlice<'r>(&'r [xdp_desc], &'r mut XdpUmemUqueue);

impl<'r> UmemReadRingSlice<'r> {
    fn from_idx(queue: &'r mut XdpUmemUqueue, ring_index: isize, entries: usize) -> Self {
        let descs =
            unsafe { std::slice::from_raw_parts(queue.ring.offset(ring_index), entries as usize) };
        Self(descs, queue)
    }
}

impl<'r> std::ops::Deref for UmemReadRingSlice<'r> {
    type Target = &'r [xdp_desc];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'r> std::ops::Drop for UmemReadRingSlice<'r> {
    fn drop(&mut self) {
        self.1.cached_consumer += self.0.len() as u32;
        unsafe {
            *self.1.consumer = self.1.cached_consumer;
        }
    }
}

impl XdpUmemUqueue {
    fn rx_available_entries(&mut self, descs_available: u32) -> u32 {
        let mut entries = self.cached_producer - self.cached_consumer;
        if entries == 0 {
            self.cached_producer = unsafe { *self.producer };
            entries = self.cached_producer - self.cached_consumer;
        }

        println!(
            "entries: {}, producer: {}, consumer: {}",
            entries, self.cached_producer, self.cached_consumer
        );




        std::cmp::min(entries, descs_available)
    }

    fn tx_available_entries(&mut self, descs_available: u32) -> u32 {
        let entries = self.cached_consumer - self.cached_producer;
        if entries >= descs_available {
            return descs_available;
        }

        self.cached_consumer = unsafe { *self.consumer } + self.size;
        return self.cached_consumer - self.cached_producer;
    }

    pub fn rx_dequeue(&mut self, batch_size: u32) -> UmemReadRingSlice {
        let entries = self.rx_available_entries(batch_size) as usize;
        let ring_index = (self.cached_consumer & self.mask) as isize;
        UmemReadRingSlice::from_idx(self, ring_index, entries)
    }

    pub fn tx_enqueue(&mut self, descs: &[xdp_desc]) -> Result<(), failure::Error> {
        let free_space = self.tx_available_entries(descs.len() as u32) as usize;
        if free_space < descs.len() {
            bail!(
                "not enough available transmit descriptors: {} < {}",
                free_space,
                descs.len()
            );
        }

        for d in descs {
            let idx = self.cached_producer & self.mask;
            self.cached_producer += 1;

            unsafe {
                let tx_spot = self.ring.offset(idx as isize);
                (*tx_spot).addr = d.addr;
                (*tx_spot).len = d.len;
            }
        }

        unsafe {
            *self.producer = self.cached_producer;
        }

        Ok(())
    }
}

pub struct XdpUmem {
    pub frames: *mut c_void,
    pub fq: XdpUmemUqueue,
    pub cq: XdpUmemUqueue,
    pub sfd: c_int,
}

impl XdpUmem {
    pub unsafe fn get_data(&self, addr: isize) -> *mut libc::c_char {
        self.frames.offset(addr) as *mut libc::c_char
    }
}

use failure::{bail, format_err};

#[macro_export]
macro_rules! xdp_mmap_regions {
    ($fd:expr,$mmap_off_flag:expr,$off_field:ident,$numdescs:expr) => {{ try {
        use failure::format_err;

        let mut optlen: libc::socklen_t = std::mem::size_of::<$crate::if_xdp::xdp_mmap_offsets>() as u32;
        let mut off: $crate::if_xdp::xdp_mmap_offsets = Default::default();
        let ok = libc::getsockopt(
            $fd,
            $crate::if_xdp::SOL_XDP,
            $crate::if_xdp::XDP_MMAP_OFFSETS as libc::c_int,
            &mut off as *mut _ as *mut libc::c_void,
            &mut optlen as *mut libc::socklen_t,
        );

        if ok < 0 {
            Err(format_err!("Could not get XDP sockopt for mmap offsets"))?
        }

        let map: *mut libc::c_void = libc::mmap(
            std::ptr::null_mut(),
            (off.fr.desc + $numdescs as u64 * std::mem::size_of::<u64>() as u64) as usize,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED | libc::MAP_POPULATE,
            $fd,
            $mmap_off_flag as libc::off_t,
        );

        if map == libc::MAP_FAILED {
            Err(format_err!("Could not mmap fill ring"))?
        }

        $crate::if_xdp::XdpUmemUqueue {
            map,
            mask: ($numdescs - 1) as u32,
            size: $numdescs as u32,
            producer: map.offset(off.$off_field.producer as isize) as *mut u32,
            consumer: map.offset(off.$off_field.consumer as isize) as *mut u32,
            ring: map.offset(off.$off_field.desc as isize) as *mut _,
            cached_consumer: Default::default(),
            cached_producer: Default::default(),
        }
    }}};
    ($fd:expr, "fill") => {{
        let fq_r: Result<$crate::if_xdp::XdpUmemUqueue, failure::Error> = xdp_mmap_regions!($fd, XDP_UMEM_PGOFF_FILL_RING, fr, FQ_NUM_DESCS);
        fq_r.map(|mut fq| {
            fq.cached_consumer = $crate::if_xdp::NUM_DESCS as u32;
            fq
        })
    }};
    ($fd:expr, "completion") => {
        xdp_mmap_regions!($fd, XDP_UMEM_PGOFF_COMPLETION_RING, cr, CQ_NUM_DESCS)
    };
    ($fd:expr, "rx") => {
        xdp_mmap_regions!($fd, $crate::if_xdp::XDP_PGOFF_RX_RING, rx, $crate::if_xdp::NUM_DESCS)
    };
    ($fd:expr, "tx") => {{
        let tx_r: Result<$crate::if_xdp::XdpUmemUqueue, failure::Error> = xdp_mmap_regions!($fd, $crate::if_xdp::XDP_PGOFF_TX_RING, tx, $crate::if_xdp::NUM_DESCS);
        tx_r.map(|mut tx| {
            tx.cached_consumer = $crate::if_xdp::NUM_DESCS as u32;
            tx
        })
    }};
}

pub unsafe fn xdp_umem_configure(sfd: c_int) -> Result<XdpUmem, failure::Error> {
    let mut buf = std::ptr::null_mut::<c_void>();
    let page_size = libc::sysconf(libc::_SC_PAGESIZE) as usize;

    // allocate the umem memory
    let ok = libc::posix_memalign(&mut buf, page_size, NUM_FRAMES * FRAME_SIZE);
    if ok < 0 {
        bail!("Could not get page-aligned pointer");
    }

    let mr = xdp_umem_reg {
        addr: buf as u64,
        len: NUM_FRAMES as u64 * FRAME_SIZE as u64,
        chunk_size: FRAME_SIZE as u32,
        headroom: FRAME_HEADROOM as u32,
    };

    // register the umem memory with the socket
    let ok = libc::setsockopt(
        sfd,
        SOL_XDP,
        XDP_UMEM_REG as c_int,
        &mr as *const _ as *const c_void,
        std::mem::size_of::<xdp_umem_reg>() as libc::socklen_t,
    );
    if ok < 0 {
        bail!("Could not set XDP sockopt to register UMEM");
    }

    let fq_size = FQ_NUM_DESCS;
    let cq_size = CQ_NUM_DESCS;

    let ok = libc::setsockopt(
        sfd,
        SOL_XDP,
        XDP_UMEM_FILL_RING as c_int,
        &fq_size as *const _ as *const c_void,
        std::mem::size_of::<c_int>() as libc::socklen_t,
    );
    if ok < 0 {
        bail!("Could not set XDP sockopt to register fill queue");
    }

    let ok = libc::setsockopt(
        sfd,
        SOL_XDP,
        XDP_UMEM_COMPLETION_RING as c_int,
        &cq_size as *const _ as *const c_void,
        std::mem::size_of::<c_int>() as libc::socklen_t,
    );
    if ok < 0 {
        bail!("Could not set XDP sockopt to register fill queue");
    }

    let fq: Result<XdpUmemUqueue, failure::Error> = xdp_mmap_regions!(sfd, "fill");
    let cq: Result<XdpUmemUqueue, failure::Error> = xdp_mmap_regions!(sfd, "completion");

    Ok(XdpUmem {
        fq: fq?,
        cq: cq?,
        frames: buf,
        sfd,
    })
}
