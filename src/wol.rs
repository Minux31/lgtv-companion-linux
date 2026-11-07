use anyhow::{bail, Context, Result};
use std::{net::{Ipv4Addr, SocketAddrV4}};
use tokio::net::UdpSocket;

pub async fn send_magic_packet(mac: &str, broadcast: &Ipv4Addr, port: u16) -> Result<()> {
    let bytes = parse_mac(mac)?;
    let mut packet = [0xffu8; 102];
    for chunk in packet[6..].chunks_exact_mut(6) {
        chunk.copy_from_slice(&bytes);
    }

    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).await?;
    socket.set_broadcast(true)?;
    let target = SocketAddrV4::new(*broadcast, port);
    let sent = socket.send_to(&packet, target).await.context("UDP send failed")?;
    anyhow::ensure!(sent == packet.len(), "incomplete magic packet sent");
    Ok(())
}

fn parse_mac(value: &str) -> Result<[u8; 6]> {
    let normalized = value.replace('-', ":");
    let parts: Vec<_> = normalized.split(':').collect();
    if parts.len() != 6 { bail!("invalid MAC address: expected six octets"); }
    let mut mac = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        mac[i] = u8::from_str_radix(part, 16)
            .with_context(|| format!("invalid MAC octet {part}"))?;
    }
    Ok(mac)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_mac_address() {
        assert_eq!(parse_mac("aa:bb:cc:dd:ee:ff").unwrap(), [0xaa,0xbb,0xcc,0xdd,0xee,0xff]);
        assert!(parse_mac("not-a-mac").is_err());
    }
}
