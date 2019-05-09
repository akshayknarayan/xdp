// libbpf rust bindings
include!(concat!(env!("OUT_DIR"), "/libbpf.rs"));

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(unused)]
mod bpf {
    // bpf rust bindings
    include!(concat!(env!("OUT_DIR"), "/bpf.rs"));
}

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(unused)]
mod if_link {
    // if_link.h bindings
    // used only for consts
    include!(concat!(env!("OUT_DIR"), "/if_link.rs"));
}

use failure::bail;

pub struct BpfHandles {
    prog_fd: std::os::raw::c_int,
    bpf_obj: *mut bpf_object,
    xsks_map: std::os::raw::c_int,
    qidconf_map: std::os::raw::c_int,
}

impl BpfHandles {
    pub fn new(interface_id: u32) -> Result<Self, failure::Error> {
        load_bpf_program(interface_id)
    }

    pub fn register_xdp_sock(
        &self,
        sock: &crate::xdp_sock::XdpSocket,
    ) -> Result<(), failure::Error> {
        let key = 0;
        let fd = sock.sfd;
        let ok = unsafe {
            bpf::bpf_map_update_elem(
                self.xsks_map,
                &key as *const _ as *const _,
                &fd as *const _ as *const _,
                0,
            )
        };
        if ok > 0 {
            bail!("bpf_map_update_elem failed: {}", ok);
        }
        Ok(())
    }
}

impl Drop for BpfHandles {
    fn drop(&mut self) {
        let ok = unsafe { bpf_object__unload(self.bpf_obj) };
        if ok < 0 {
            panic!("didn't find object?? {:?}", ok);
        }
    }
}

macro_rules! as_raw_str {
    ($s: expr) => {{
        std::ffi::CStr::from_bytes_with_nul($s.as_bytes())
    }};
}

fn get_map_by_name(
    name: &str,
    bpf_obj: *mut bpf_object,
) -> Result<std::os::raw::c_int, failure::Error> {
    let map_name_str = as_raw_str!(name)?;
    let map = unsafe {
        let map = bpf_object__find_map_by_name(bpf_obj, map_name_str.as_ptr());
        bpf_map__fd(map)
    };

    if map < 0 {
        bail!("{} map not found", name);
    }

    Ok(map)
}

fn load_bpf_program(interface_id: u32) -> Result<BpfHandles, failure::Error> {
    let bpf_filename = concat!(env!("OUT_DIR"), "/xdp-bpf.o\0");
    let bpf_filename_cstr = as_raw_str!(bpf_filename)?;
    let attr = bpf_prog_load_attr {
        file: bpf_filename_cstr.as_ptr(),
        prog_type: bpf_prog_type_BPF_PROG_TYPE_XDP,
        expected_attach_type: bpf_attach_type_BPF_CGROUP_INET_INGRESS,
        ifindex: 0,
        log_level: 0,
    };

    let mut bpf_obj: *mut bpf_object = std::ptr::null_mut();
    let mut prog_fd = 0;

    let ok = unsafe {
        bpf_prog_load_xattr(
            &attr,
            &mut bpf_obj as *mut *mut bpf_object,
            &mut prog_fd as *mut _,
        )
    };
    if ok > 0 {
        bail!("bpf_prog_load_xattr failed: {}", ok);
    }

    if prog_fd == 0 {
        bail!("bpf_prog_load_xattr returned null fd");
    }

    if prog_fd < 0 {
        bail!("bpf_prog_load_xattr returned bad fd: {}", prog_fd);
    }

    let qidconf_map = get_map_by_name("qidconf_map\0", bpf_obj)?;
    let xsks_map = get_map_by_name("xsks_map\0", bpf_obj)?;

    let xdp_flags = if_link::XDP_FLAGS_SKB_MODE;
    let ok = unsafe { bpf_set_link_xdp_fd(interface_id as i32, prog_fd, xdp_flags) };
    if ok < 0 {
        bail!("bpf_set_link_xdp_fd failed: {}", ok);
    }

    // set qidconf to use NIC queue 0
    let key = 0;
    let qid = 0;
    let ok = unsafe {
        bpf::bpf_map_update_elem(
            qidconf_map,
            &key as *const _ as *const _,
            &qid as *const _ as *const _,
            0,
        )
    };
    if ok > 0 {
        bail!("bpf_map_update_elem failed: {}", ok);
    }

    Ok(BpfHandles {
        prog_fd,
        bpf_obj,
        xsks_map,
        qidconf_map,
    })
}
