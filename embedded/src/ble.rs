use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering},
    },
};

use anyhow::Result;
use clone_macro::clone;
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{
    bt::{
        Ble, BtDriver, BtUuid,
        ble::{
            gap::{AdvConfiguration, AppearanceCategory, BleGapEvent, EspBleGap},
            gatt::{
                AutoResponse, GattCharacteristic, GattDescriptor, GattId, GattResponse,
                GattServiceId, GattStatus, Permission, Property,
                server::{EspGatts, GattsEvent},
            },
        },
    },
    nvs::EspDefaultNvsPartition,
};
use log::info;
use uuid::uuid;

use characteristics::CharacteristicHandles;

use crate::{app::App, ble::characteristics::Characteristic};

const APP_ID: u16 = 0;
const SERVICE: u128 = uuid!("76e20500-da73-4971-bb03-6105e39db3d6").as_u128();

pub mod characteristics {
    use std::sync::atomic::{AtomicU16, Ordering};

    use esp_idf_svc::bt::BtUuid;
    use uuid::uuid;

    pub const ALL: &[u128] = &[POSITION, SPEED, WIND];

    pub const POSITION: u128 = uuid!("300b2aec-a094-43fb-98ff-04917cf7a2fb").as_u128();
    pub const SPEED: u128 = uuid!("d948b9e5-6626-4d41-8967-c4dca26db1fd").as_u128();
    pub const WIND: u128 = uuid!("91331b1c-3132-4197-aa43-b86b7df421f1").as_u128();

    #[derive(Debug, PartialEq, Eq, Hash)]
    pub enum Characteristic {
        Position,
        Speed,
        Wind,
    }

    #[derive(Default)]
    pub struct CharacteristicHandles {
        pub position: AtomicU16,
        pub speed: AtomicU16,
        pub wind: AtomicU16,
    }

    impl CharacteristicHandles {
        pub fn init(&self, char_uuid: BtUuid, attr_handle: u16) {
            let uuid = u128::from_ne_bytes(*char_uuid.as_bytes().as_array::<16>().unwrap());
            match uuid {
                POSITION => self.position.store(attr_handle, Ordering::Relaxed),
                SPEED => self.speed.store(attr_handle, Ordering::Relaxed),
                WIND => self.wind.store(attr_handle, Ordering::Relaxed),
                _ => unreachable!(),
            }
        }

        pub fn characteristic(&self, handle: u16) -> Option<Characteristic> {
            if handle == self.position.load(Ordering::Relaxed) {
                Some(Characteristic::Position)
            } else if handle == self.speed.load(Ordering::Relaxed) {
                Some(Characteristic::Speed)
            } else if handle == self.wind.load(Ordering::Relaxed) {
                Some(Characteristic::Wind)
            } else {
                None
            }
        }

        pub fn handle(&self, characteristic: &Characteristic) -> u16 {
            match characteristic {
                Characteristic::Position => self.position.load(Ordering::Relaxed),
                Characteristic::Speed => self.speed.load(Ordering::Relaxed),
                Characteristic::Wind => self.wind.load(Ordering::Relaxed),
            }
        }
    }
}

pub struct Bluetooth {
    gap: EspBleGap<'static, Ble, Arc<BtDriver<'static, Ble>>>,
    gatts: EspGatts<'static, Ble, Arc<BtDriver<'static, Ble>>>,
    gatts_if: AtomicU8,

    handles: CharacteristicHandles,
    clients: Mutex<HashMap<u16, Client>>,
}

#[derive(Default)]
struct Client {
    subscribed: HashSet<Characteristic>,
}

pub fn init(app: Arc<App>, modem: Modem<'static>) -> Result<()> {
    let nvs = EspDefaultNvsPartition::take()?;
    let driver = Arc::new(BtDriver::<Ble>::new(modem, Some(nvs))?);

    let bt = Arc::new(Bluetooth {
        gap: EspBleGap::new(driver.clone())?,
        gatts: EspGatts::new(driver.clone())?,
        gatts_if: AtomicU8::new(0),
        handles: CharacteristicHandles::default(),
        clients: Mutex::new(HashMap::new()),
    });

    bt.gap.subscribe(clone!([bt], move |event| {
        if let BleGapEvent::AdvertisingConfigured(_) = event {
            bt.gap.start_advertising().unwrap();
            info!("Advertising started");
        }

        if let BleGapEvent::AdvertisingConfigured(_) = event {}
    }))?;

    bt.gap.set_device_name("windlink").unwrap();
    bt.gap
        .set_adv_conf(&AdvConfiguration {
            include_name: false,
            include_txpower: true,
            flag: 0x06,
            service_uuid: Some(BtUuid::uuid128(SERVICE)),
            appearance: AppearanceCategory::NetworkDevice,
            ..Default::default()
        })
        .unwrap();

    bt.gatts
        .subscribe(clone!([app, bt], move |(gatt_if, event)| match event {
            GattsEvent::ServiceRegistered { app_id, .. } if app_id == APP_ID => {
                bt.gatts_if.store(gatt_if, Ordering::Relaxed);
                let service = GattServiceId {
                    id: GattId {
                        uuid: BtUuid::uuid128(SERVICE),
                        inst_id: 0,
                    },
                    is_primary: true,
                };
                bt.gatts.create_service(gatt_if, &service, 10).unwrap();
            }
            GattsEvent::ServiceCreated { service_handle, .. } => {
                bt.gatts.start_service(service_handle).unwrap();

                for &uuid in characteristics::ALL {
                    let characteristic = GattCharacteristic {
                        uuid: BtUuid::uuid128(uuid),
                        permissions: Permission::Read.into(),
                        properties: Property::Read | Property::Notify,
                        max_len: 20,
                        auto_rsp: AutoResponse::ByApp,
                    };
                    bt.gatts
                        .add_characteristic(service_handle, &characteristic, &[])
                        .unwrap();

                    let ccc_descriptor = GattDescriptor {
                        uuid: BtUuid::uuid16(0x2902),
                        permissions: Permission::Read | Permission::Write,
                    };
                    bt.gatts
                        .add_descriptor(service_handle, &ccc_descriptor)
                        .unwrap();
                }
            }
            GattsEvent::CharacteristicAdded {
                attr_handle,
                char_uuid,
                ..
            } => {
                bt.handles.init(char_uuid, attr_handle);
            }
            GattsEvent::PeerConnected { conn_id, .. } => {
                bt.clients
                    .lock()
                    .unwrap()
                    .insert(conn_id, Client::default());
                bt.gap.start_advertising().unwrap();
            }
            GattsEvent::PeerDisconnected { conn_id, .. } => {
                bt.clients.lock().unwrap().remove(&conn_id);
                bt.gap.start_advertising().unwrap();
            }
            GattsEvent::Write {
                conn_id,
                trans_id,
                handle,
                value,
                ..
            } => {
                let mut clients = bt.clients.lock().unwrap();
                if let Some(client) = clients.get_mut(&conn_id)
                    && let Some(characteristic) = bt.handles.characteristic(handle - 1)
                {
                    if value == [1, 0] {
                        client.subscribed.insert(characteristic);
                    } else if value == [0, 0] {
                        client.subscribed.remove(&characteristic);
                    }

                    bt.gatts
                        .send_response(gatt_if, conn_id, trans_id, GattStatus::Ok, None)
                        .unwrap();
                }
            }
            GattsEvent::Read {
                conn_id,
                trans_id,
                handle,
                ..
            } => {
                let Some(characteristic) = bt.handles.characteristic(handle) else {
                    return;
                };

                let mut response = GattResponse::new();
                response.attr_handle(handle);
                response.value(&app.boat().packet(characteristic)).unwrap();

                bt.gatts
                    .send_response(gatt_if, conn_id, trans_id, GattStatus::Ok, Some(&response))
                    .unwrap();
            }
            _ => {}
        }))?;

    bt.gatts.register_app(APP_ID)?;
    app.bt.lock().unwrap().replace(bt);
    info!("Initialized BLE");
    Ok(())
}

impl Bluetooth {
    fn gatts_if(&self) -> u8 {
        self.gatts_if.load(Ordering::Relaxed)
    }

    pub fn notify(&self, characteristic: Characteristic, data: &[u8]) {
        let attr_handle = self.handles.handle(&characteristic);
        let clients = self.clients.lock().unwrap();

        for (&conn_id, client) in clients.iter() {
            if client.subscribed.contains(&characteristic) {
                self.gatts
                    .indicate(self.gatts_if(), conn_id, attr_handle, data)
                    .unwrap();
            }
        }
    }
}
