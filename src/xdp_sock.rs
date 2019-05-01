use crate::if_xdp;
use crate::libbpf;
use failure::bail;

pub struct XdpSocket {
    rx: if_xdp::XdpUmemUqueue,
    tx: if_xdp::XdpUmemUqueue,
    pub sfd: libc::c_int,
    umem: if_xdp::XdpUmem,
}

impl XdpSocket {
    pub fn new(interface: &str) -> Result<Self, failure::Error> {
        let interface_id = crate::get_interface_id(interface)?;
        let bpf = libbpf::BpfHandles::new(interface_id)?;
        let sk = unsafe { xdp_configure(interface_id, 0) }?;
        bpf.register_xdp_sock(&sk)?;
        Ok(sk)
    }

    pub fn read_batch(&mut self, batch_size: u32) {
        let rx_frames = self.rx.rx_dequeue(batch_size);
        if rx_frames.is_empty() {
            println!("no frames available");
            return;
        }

        for f in *rx_frames {
            let data_addr = unsafe { self.umem.get_data(f.addr as isize) } as *mut libc::c_char;
            let data_len = f.len as usize;
            let data = unsafe { std::slice::from_raw_parts(data_addr, data_len) };
            println!("{:?}", data);
        }
    }
}

unsafe fn xdp_configure(interface_index: u32, queue_id: u32) -> Result<XdpSocket, failure::Error> {
    let sfd = libc::socket(if_xdp::PF_XDP, libc::SOCK_RAW, 0);
    if sfd < 0 {
        bail!("Could not initialize XDP socket");
    }

    let umem = if_xdp::xdp_umem_configure(sfd)?;

    let ok = libc::setsockopt(
        sfd,
        if_xdp::SOL_XDP,
        if_xdp::XDP_RX_RING as i32,
        &if_xdp::NUM_DESCS as *const _ as *const libc::c_void,
        std::mem::size_of::<libc::c_int>() as u32,
    );

    if ok < 0 {
        bail!("setsockopt on RX_RING failed");
    }

    let ok = libc::setsockopt(
        sfd,
        if_xdp::SOL_XDP,
        if_xdp::XDP_TX_RING as i32,
        &if_xdp::NUM_DESCS as *const _ as *const libc::c_void,
        std::mem::size_of::<libc::c_int>() as u32,
    );

    if ok < 0 {
        bail!("setsockopt on RX_RING failed");
    }

    let rx: Result<if_xdp::XdpUmemUqueue, failure::Error> = xdp_mmap_regions!(sfd, "rx");
    let rx = rx?;
    let tx: Result<if_xdp::XdpUmemUqueue, failure::Error> = xdp_mmap_regions!(sfd, "tx");
    let tx = tx?;

    let mut addr = if_xdp::sockaddr_xdp::default();
    addr.sxdp_family = if_xdp::PF_XDP as u16;
    addr.sxdp_ifindex = interface_index;
    addr.sxdp_queue_id = queue_id;
    let ok = libc::bind(
        sfd,
        &mut addr as *mut _ as *mut libc::sockaddr,
        std::mem::size_of::<if_xdp::sockaddr_xdp>() as u32,
    );

    if ok > 0 {
        bail!("Could not bind xdp socket: {}", ok);
    }

    Ok(XdpSocket { rx, tx, sfd, umem })
}
