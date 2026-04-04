pub fn create_udp_listener(bind_addr: SocketAddr) -> io::Result<UdpSocket> {
    let domain = if bind_addr.is_ipv6() { Domain::IPV6 } else { Domain::IPV4 };

    let sock = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
    sock.set_reuse_address(true)?;
    sock.bind(&bind_addr.into())?;

    sock.set_nonblocking(true)?;
    let fd: OwnedFd = sock.into();
    let std_sock: std::net::UdpSocket = fd.into();
    return UdpSocket::from_std(std_sock);
}

pub fn create_connected_udp_socket(
    bind_addr: SocketAddr,
    connect_addr: SocketAddr,
) -> io::Result<UdpSocket> {
    let domain = if bind_addr.is_ipv6() { Domain::IPV6 } else { Domain::IPV4 };

    let sock = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
    sock.set_reuse_address(true)?;
    sock.bind(&bind_addr.into())?;
    sock.connect(&connect_addr.into())?;

    sock.set_nonblocking(true)?;
    let fd: OwnedFd = sock.into();
    let std_sock: std::net::UdpSocket = fd.into();
    return UdpSocket::from_std(std_sock);
}

//firefox does not accept loopback ips for webrtc when testing locally
pub fn local_network_ip() -> Ipv4Addr {
    let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") else {
        return Ipv4Addr::LOCALHOST;
    };
    if sock.connect("192.0.2.1:80").is_err() {
        return Ipv4Addr::LOCALHOST;
    }
    let Ok(addr) = sock.local_addr() else {
        return Ipv4Addr::LOCALHOST;
    };
    let IpAddr::V4(ip) = addr.ip() else {
        return Ipv4Addr::LOCALHOST;
    };
    return ip;
}

use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::fd::OwnedFd;
use tokio::net::UdpSocket;
