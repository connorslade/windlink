use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering},
    },
};

use anyhow::Result;
use clone_macro::clone;
use esp_idf_hal::{
    modem::BluetoothModem,
    sys::{
        esp_ble_power_type_t_ESP_BLE_PWR_TYPE_ADV as ESP_BLE_PWR_TYPE_ADV,
        esp_ble_power_type_t_ESP_BLE_PWR_TYPE_DEFAULT as ESP_BLE_PWR_TYPE_DEFAULT,
        esp_ble_power_type_t_ESP_BLE_PWR_TYPE_SCAN as ESP_BLE_PWR_TYPE_SCAN, esp_ble_tx_power_set,
        esp_power_level_t_ESP_PWR_LVL_P9 as ESP_PWR_LVL_P9,
    },
};
use esp_idf_svc::bt::{
    Ble, BtDriver, BtUuid,
    ble::{
        gap::{AdvConfiguration, AppearanceCategory, BleGapEvent, EspBleGap},
        gatt::{
            AutoResponse, GattCharacteristic, GattDescriptor, GattId, GattResponse, GattServiceId,
            GattStatus, Permission, Property,
            server::{EspGatts, GattsEvent},
        },
    },
};
use log::info;
use uuid::uuid;

use characteristics::CharacteristicHandles;

use crate::{app::App, ble::characteristics::Characteristic, util::ForceLock};

const APP_ID: u16 = 0;
const SERVICE: u128 = uuid!("76e20500-da73-4971-bb03-6105e39db3d6").as_u128();

pub mod characteristics {
    use std::sync::atomic::{AtomicU16, Ordering};

    use esp_idf_svc::bt::BtUuid;
    use uuid::uuid;

    pub const ALL: &[u128] = &[WIND_SCREEN];

    pub const WIND_SCREEN: u128 = uuid!("300b2aec-a094-43fb-98ff-04917cf7a2fb").as_u128();

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Characteristic {
        WindScreen,
    }

    #[derive(Default)]
    pub struct CharacteristicHandles {
        pub wind_screen: AtomicU16,
    }

    impl CharacteristicHandles {
        pub fn init(&self, char_uuid: BtUuid, attr_handle: u16) {
            let uuid = u128::from_ne_bytes(*char_uuid.as_bytes().as_array::<16>().unwrap());
            match uuid {
                WIND_SCREEN => self.wind_screen.store(attr_handle, Ordering::Relaxed),
                _ => unreachable!(),
            }
        }

        pub fn characteristic(&self, handle: u16) -> Option<Characteristic> {
            if handle == self.wind_screen.load(Ordering::Relaxed) {
                Some(Characteristic::WindScreen)
            } else {
                None
            }
        }

        pub fn handle(&self, characteristic: &Characteristic) -> u16 {
            match characteristic {
                Characteristic::WindScreen => self.wind_screen.load(Ordering::Relaxed),
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

pub fn init(app: Arc<App>, modem: BluetoothModem<'static>) -> Result<()> {
    let driver = Arc::new(BtDriver::<Ble>::new(modem, Some(app.nvs.clone()))?);
    unsafe { esp_ble_tx_power_set(ESP_BLE_PWR_TYPE_DEFAULT, ESP_PWR_LVL_P9) };
    unsafe { esp_ble_tx_power_set(ESP_BLE_PWR_TYPE_SCAN, ESP_PWR_LVL_P9) };
    unsafe { esp_ble_tx_power_set(ESP_BLE_PWR_TYPE_ADV, ESP_PWR_LVL_P9) };

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
                        properties: Property::Read | Property::Indicate,
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
                bt.clients.force_lock().insert(conn_id, Client::default());
                bt.gap.start_advertising().unwrap();
            }
            GattsEvent::PeerDisconnected { conn_id, .. } => {
                bt.clients.force_lock().remove(&conn_id);
                bt.gap.start_advertising().unwrap();
            }
            GattsEvent::Write {
                conn_id,
                trans_id,
                handle,
                value,
                ..
            } => {
                let mut clients = bt.clients.force_lock();
                if let Some(client) = clients.get_mut(&conn_id)
                    && let Some(characteristic) = bt.handles.characteristic(handle - 1)
                {
                    if matches!(value, [1, 0] | [2, 0]) {
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
    app.bt.force_lock().replace(bt);
    info!("Initialized BLE");
    Ok(())
}

impl Bluetooth {
    fn gatts_if(&self) -> u8 {
        self.gatts_if.load(Ordering::Relaxed)
    }

    pub fn notify(&self, characteristic: Characteristic, data: &[u8]) {
        let attr_handle = self.handles.handle(&characteristic);
        let clients = self.clients.force_lock();

        for (&conn_id, client) in clients.iter() {
            if client.subscribed.contains(&characteristic) {
                self.gatts
                    .indicate(self.gatts_if(), conn_id, attr_handle, data)
                    .unwrap();
            }
        }
    }
}
