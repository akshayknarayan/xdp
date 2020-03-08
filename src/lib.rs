#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(unused)]
#[macro_use]
mod if_xdp;

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(unused)]
mod libbpf;

pub mod xdp_sock;

// Useful XDP linksa
// https://github.com/xdp-project/xdp-tutorial/tree/master/packet02-rewriting
// https://github.com/xdp-project/xdp-tutorial/tree/master/advanced03-AF_XDP
//
// https://lwn.net/Articles/750845/
// https://archive.fosdem.org/2018/schedule/event/af_xdp/attachments/slides/2221/export/events/attachments/af_xdp/slides/2221/fosdem_2018_v3.pdf
// https://www.kernel.org/doc/html/latest/networking/af_xdp.html

fn get_interface_id(interface_name: &str) -> nix::Result<u32> {
    nix::net::if_::if_nametoindex(interface_name)
}
