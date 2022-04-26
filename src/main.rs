use async_lock::Semaphore;
use gumdrop::Options;
use local_ip_address::local_ip;
use socket2::{Socket, Type};
use std::mem::MaybeUninit;
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::thread;
use std::time;

#[derive(Debug, Options)]
pub struct Args {
    #[options(help = "print help message")]
    help: bool,
    #[options(free)]
    free: Vec<String>,
    #[options(help = "milliseconds to try", default = "50")]
    expiry: u64,
    #[options(help = "number of jobs", meta = "N", default = "10")]
    jobs: usize,
}

#[tokio::main]
async fn main() {
    let args = Args::parse_args_default_or_exit();
    let Args {
        free: _,
        expiry,
        jobs,
        ..
    } = args;

    env_logger::init();

    let my_local_ip = local_ip().unwrap();
    let s = Arc::new(Semaphore::new(jobs));

    if let IpAddr::V4(my_local_ip) = my_local_ip {
        // println!("This is my local IP address: {:?}", my_local_ip);
        let octets = my_local_ip.octets();
        let mut handles = vec![];
        for n in 1..255 {
            let ip = Ipv4Addr::from([octets[0], octets[1], octets[2], n]);
            let guard = s.acquire_arc().await;
            let handle = thread::spawn(move || {
                try_rtsp(ip, expiry);
                drop(guard);
            });
            handles.push(handle);
        }
        for handle in handles {
            handle.join().unwrap();
        }
    }
}

fn try_rtsp(ip: Ipv4Addr, expiry: u64) {
    let to = time::Duration::from_millis(expiry);
    let socket = SocketAddrV4::new(ip, 554);
    let s = Socket::new(socket2::Domain::IPV4, Type::STREAM, None).unwrap();
    s.set_read_timeout(Some(to)).unwrap();
    let r = s.connect_timeout(&socket.into(), to);
    if let Err(_) = r {
        return;
    }
    let data = format!(
        "OPTIONS rtsp://{}/ RTSP/1.0\r\nCSeq: 1\r\nUser-Agent: VLC media player\r\n\r\n",
        ip.to_string()
    );
    s.send(&data.as_bytes()).unwrap();
    let mut buf = [MaybeUninit::<u8>::uninit(); 1024];
    let n = s.recv(&mut buf).unwrap_or(0);
    if n > 0 {
        println!("rtsp://{}:554/", ip);
    }
    // println!("{:?}", n);
    // let x = buf[..n]
    //     .iter()
    //     .map(|x| unsafe { x.assume_init() })
    //     .collect::<Vec<u8>>();
    // println!("{:?}", String::from_utf8_lossy(x.as_slice()));
}
