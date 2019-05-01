#![feature(try_blocks)]

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

fn get_interface_id(interface_name: &str) -> nix::Result<u32> {
    nix::net::if_::if_nametoindex(interface_name)
}
