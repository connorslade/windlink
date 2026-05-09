use std::{
    io::{BufReader, BufWriter, Read, Write, stdin},
    net::{SocketAddr, TcpStream},
    sync::mpsc::sync_channel,
    thread,
};

use anyhow::{Result, bail};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use nmea2000::{
    Header, Nmea2000,
    packets::{Packet, proprietary::SimnetAp},
};

fn main() -> Result<()> {
    println!("Searching for windlink");
    let service = find_service()?;
    println!("Found!");

    let socket = TcpStream::connect(service)?;
    let mut reader = BufReader::new(socket.try_clone()?);
    let mut writer = BufWriter::new(socket);

    let mut nmea2000 = Nmea2000::new();

    let (tx, rx) = sync_channel(10);
    thread::spawn(move || {
        loop {
            let out = &mut [0];
            if let Ok(1) = stdin().read(out)
                && out[0] == 10
            {
                tx.send(Packet::SimnetAp(SimnetAp {
                    address: 6,
                    proprietary: 255,
                    command: 10,
                    event: 6,
                }))
                .unwrap();
            }
        }
    });

    loop {
        let ident = u32::from_be_bytes(read_bytes::<4>(&mut reader)?);
        let header = Header::deserialize(ident);

        let mut data = [0_u8; 8];
        let length = read_bytes::<1>(&mut reader)?[0] as usize;
        reader.read_exact(&mut data[..length])?;

        if header.pgn == SimnetAp::PGN {
            println!(" | {header:?} {:?}", &data[..length]);
        }

        if let Some(Packet::SimnetAp(packet)) = nmea2000.on_packet(ident, data) {
            println!("{packet:?}");
        }

        while let Ok(packet) = rx.try_recv() {
            println!("enqueue!");
            nmea2000.enqueue(packet, 255);
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
