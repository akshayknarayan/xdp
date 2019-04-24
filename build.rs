use std::env;
use std::path::PathBuf;

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    std::process::Command::new("make")
        .current_dir("./src/libbpf/src")
        .spawn()
        .expect("libbpf spawn make")
        .wait()
        .expect("libbpf spawn make");

    let libbpf_bindings = bindgen::Builder::default()
        .header("./src/libbpf/src/libbpf.h")
        .whitelist_function("bpf_prog_load_xattr")
        .whitelist_function("bpf_object__find_map_by_name")
        .whitelist_function("bpf_map__fd")
        .whitelist_function("bpf_set_link_xdp_fd")
        .blacklist_type(r#"u\d+"#)
        .generate()
        .expect("Unable to generate bindings");
    libbpf_bindings
        .write_to_file(out_path.join("libbpf.rs"))
        .expect("Unable to write bindings");

    println!("cargo:rustc-link-search=./src/libbpf/src/");
    println!("cargo:rustc-link-lib=static=bpf");
    println!("cargo:rustc-link-lib=elf");

    let bpf_bindings = bindgen::Builder::default()
        .header("./src/libbpf/src/bpf.h")
        .whitelist_function("bpf_map_update_elem")
        .blacklist_type(r#"u\d+"#)
        .generate()
        .expect("Unable to generate bindings");
    bpf_bindings
        .write_to_file(out_path.join("bpf.rs"))
        .expect("Unable to write bindings");

    std::process::Command::new("make")
        .current_dir("./src/xdp-ebpf")
        .arg("clean")
        .spawn()
        .expect("kernel ebpf program spawn make clean")
        .wait()
        .expect("kernel ebpf program spawn make clean");
    std::process::Command::new("make")
        .current_dir("./src/xdp-ebpf")
        .spawn()
        .expect("kernel ebpf program spawn make")
        .wait()
        .expect("kernel ebpf program spawn make");

    std::process::Command::new("mv")
        .current_dir("./src/xdp-ebpf")
        .arg("xdp-bpf.o")
        .arg(out_path.join("xdp-bpf.o"))
        .spawn()
        .expect("kernel ebpf program move to outdir")
        .wait()
        .expect("kernel ebpf program move to outdir");

    let if_link_bindings = bindgen::Builder::default()
        .header("./src/libbpf/include/uapi/linux/if_link.h")
        .whitelist_var("XDP_FLAGS_.*")
        .blacklist_type(r#"u\d+"#)
        .generate()
        .expect("Unable to generate bindings");
    if_link_bindings
        .write_to_file(out_path.join("if_link.rs"))
        .expect("Unable to write bindings");

    let if_xdp_bindings = bindgen::Builder::default()
        .header("./src/libbpf/include/uapi/linux/if_xdp.h")
        .derive_default(true)
        .blacklist_type(r#"u\d+"#)
        .generate()
        .expect("Unable to generate bindings");
    if_xdp_bindings
        .write_to_file(out_path.join("if_xdp.rs"))
        .expect("Unable to write bindings");
}
