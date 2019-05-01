use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short = "i", long = "interface")]
    interface: String,
}

use xdp::xdp_sock;

fn main() -> Result<(), failure::Error> {
    let opt = Opt::from_args();

    let mut xsk = xdp_sock::XdpSocket::new(&opt.interface)?;

    let mut pollfds = libc::pollfd {
        fd: xsk.sfd,
        events: libc::POLLIN,
        revents: 0,
    };

    let ok = unsafe { libc::poll(&mut pollfds as *mut _, 1, 10000) };
    if ok < 0 {
        failure::bail!("poll returned {:?}", ok);
    }

    xsk.read_batch(128);

    Ok(())
}
