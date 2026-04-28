use std::sync::{Arc, MappedMutexGuard, Mutex, MutexGuard, mpsc::SyncSender};

use crate::{
    ble::{Bluetooth, characteristics::Characteristic},
    util::ForceLock,
};

type Soon<T> = Mutex<Option<T>>;

#[derive(Default)]
pub struct App {
    pub bt: Soon<Arc<Bluetooth>>,
    pub indicator: Soon<SyncSender<IndicatorEvent>>,

    boat: Mutex<Boat>,
}

#[derive(Default)]
pub struct Boat {
    pub latitude: i32,
    pub longitude: i32,
    pub wind_speed: u16,
    pub wind_angle: u16,
    pub speed_over_ground: u16,
}

pub enum IndicatorEvent {
    CanOnline,
}

impl App {
    pub fn boat(&self) -> MutexGuard<'_, Boat> {
        self.boat.force_lock()
    }

    pub fn bt(&self) -> MappedMutexGuard<'_, Arc<Bluetooth>> {
        MutexGuard::map(self.bt.force_lock(), |x| x.as_mut().unwrap())
    }

    pub fn indicator(&self, event: IndicatorEvent) {
        let mut channel = self.indicator.force_lock();
        channel.as_mut().unwrap().send(event).unwrap();
    }

    pub fn position_update(&self, lat: i32, lon: i32) {
        let mut boat = self.boat();
        boat.latitude = lat;
        boat.longitude = lon;
    }

    pub fn speed_update(&self, speed: u16) {
        let mut boat = self.boat();
        boat.speed_over_ground = speed;
        boat.notify(self.bt(), Characteristic::WindScreen);
    }

    pub fn wind_update(&self, speed: u16, angle: u16) {
        let mut boat = self.boat();
        boat.wind_speed = speed;
        boat.wind_angle = angle;
        boat.notify(self.bt(), Characteristic::WindScreen);
    }
}

impl Boat {
    pub fn notify(&self, bt: MappedMutexGuard<Arc<Bluetooth>>, characteristic: Characteristic) {
        bt.notify(characteristic, &self.packet(characteristic));
    }

    pub fn packet(&self, characteristic: Characteristic) -> Vec<u8> {
        match characteristic {
            Characteristic::WindScreen => self.wind_screen_packet(),
        }
    }

    fn wind_screen_packet(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(self.speed_over_ground.to_le_bytes());
        out.extend(self.wind_speed.to_le_bytes());
        out.extend(self.wind_angle.to_le_bytes());
        out
    }
}
