extern crate libc;
#[macro_use]
extern crate failure;

use libc::{c_int, c_void};

mod if_xdp;
use if_xdp::*;

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(unused)]
mod libbpf;

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
    pub cached_prod: u32,
    pub cached_conns: u32,
    pub mask: u32,
    pub size: u32,
    pub producer: *mut u32,
    pub consumer: *mut u32,
    pub ring: *mut u32,
    pub map: *mut c_void,
}

pub struct XdpUmem {
    pub frames: *mut c_void,
    pub fq: XdpUmemUqueue,
    pub cq: XdpUmemUqueue,
    pub sfd: c_int,
}

unsafe fn xdp_umem_configure(sfd: c_int) -> Result<XdpUmem, failure::Error> {
    let mut buf = std::ptr::null_mut::<c_void>();
    let page_size = libc::sysconf(libc::_SC_PAGESIZE) as usize;

    // allocate the umem memory
    let ok = libc::posix_memalign(&mut buf, page_size, NUM_FRAMES * FRAME_SIZE);
    if ok < 0 {
        bail!("Could not get page-aligned pointer");
    }

    let mr = XdpUmemReg {
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
        std::mem::size_of::<XdpUmemReg>() as libc::socklen_t,
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

    let mut optlen: libc::socklen_t = 0;
    let mut off: XdpMmapOffsets = Default::default();
    let ok = libc::getsockopt(
        sfd,
        SOL_XDP,
        XDP_MMAP_OFFSETS as c_int,
        &mut off as *mut _ as *mut c_void,
        &mut optlen as *mut libc::socklen_t,
    );
    if ok < 0 {
        bail!("Could not get XDP sockopt for mmap offsets");
    }

    let fq_map: *mut c_void = libc::mmap(
        std::ptr::null_mut(),
        (off.fr.desc + FQ_NUM_DESCS as u64 * std::mem::size_of::<u64>() as u64) as usize,
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_SHARED | libc::MAP_POPULATE,
        sfd,
        XDP_UMEM_PGOFF_FILL_RING as libc::off_t,
    );
    if fq_map == libc::MAP_FAILED {
        bail!("Could not mmap fill ring");
    }

    let cq_map: *mut c_void = libc::mmap(
        std::ptr::null_mut(),
        (off.cr.desc + CQ_NUM_DESCS as u64 * std::mem::size_of::<u64>() as u64) as usize,
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_SHARED | libc::MAP_POPULATE,
        sfd,
        XDP_UMEM_PGOFF_COMPLETION_RING as libc::off_t,
    );
    if cq_map == libc::MAP_FAILED {
        bail!("Could not mmap completion ring");
    }

    Ok(XdpUmem {
        fq: XdpUmemUqueue {
            map: fq_map,
            mask: (FQ_NUM_DESCS - 1) as u32,
            size: FQ_NUM_DESCS as u32,
            producer: fq_map.offset(off.fr.producer as isize) as *mut u32,
            consumer: fq_map.offset(off.fr.consumer as isize) as *mut u32,
            ring: fq_map.offset(off.fr.desc as isize) as *mut u32,
            cached_conns: FQ_NUM_DESCS as u32,
            cached_prod: Default::default(),
        },
        cq: XdpUmemUqueue {
            map: cq_map,
            mask: (CQ_NUM_DESCS - 1) as u32,
            size: CQ_NUM_DESCS as u32,
            producer: cq_map.offset(off.cr.producer as isize) as *mut u32,
            consumer: cq_map.offset(off.cr.consumer as isize) as *mut u32,
            ring: cq_map.offset(off.cr.desc as isize) as *mut u32,
            cached_conns: CQ_NUM_DESCS as u32,
            cached_prod: Default::default(),
        },
        frames: buf,
        sfd,
    })
}

pub unsafe fn xdp_configure() -> Result<c_int, failure::Error> {
    let sfd = libc::socket(PF_XDP, libc::SOCK_RAW, 0);
    if sfd < 0 {
        bail!("Could not initialize XDP socket");
    }

    let umem = xdp_umem_configure(sfd)?;

    Ok(sfd)
}
