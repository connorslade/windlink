use std::{
    io::{BufReader, BufWriter, Read, Write},
    net::{SocketAddr, TcpStream},
};

use anyhow::{Result, bail};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use nmea2000::Nmea2000;

fn main() -> Result<()> {
    println!("Searching for widnlink");
    let service = find_service()?;
    println!("Found!");

    let socket = TcpStream::connect(service)?;
    let mut reader = BufReader::new(socket.try_clone()?);
    let mut writer = BufWriter::new(socket);

    let mut nmea2000 = Nmea2000::new();

    loop {
        let ident = u32::from_be_bytes(read_bytes::<4>(&mut reader)?);

        let mut data = [0_u8; 8];
        let length = read_bytes::<1>(&mut reader)?[0] as usize;
        reader.read_exact(&mut data[..length])?;

        if let Some(packet) = nmea2000.on_packet(ident, data) {
            println!("{packet:?}");
        }

        for packet in nmea2000.dequeue() {
            println!("Sending {packet:?}");
            writer.write_all(&packet.id.to_be_bytes())?;
            writer.write_all(&[8])?;
            writer.write_all(&packet.data)?;
        }
    }
}

fn find_service() -> Result<SocketAddr> {
    let mdns = ServiceDaemon::new()?;
    let receiver = mdns.browse("_windlink._tcp.local.")?;

    while let Ok(event) = receiver.recv() {
        match event {
            ServiceEvent::ServiceResolved(service) => {
                let ip = service.addresses.iter().next().unwrap().to_ip_addr();
                return Ok(SocketAddr::new(ip, service.port));
            }
            _ => {}
        }
    }

    bail!("Couldn't find service")
}

fn read_bytes<const N: usize>(mut reader: impl Read) -> Result<[u8; N]> {
    let mut out = [0; N];
    reader.read_exact(&mut out)?;
    Ok(out)
}
