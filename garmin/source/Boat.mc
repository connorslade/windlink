using Toybox.BluetoothLowEnergy as Ble;
import Toybox.Lang;
import Toybox.System;
import Toybox.WatchUi;

const DEVICE_NAME = "windlink";
const SERVICE = Ble.stringToUuid("76e20500-da73-4971-bb03-6105e39db3d6");
const WIND_SCREEN = Ble.stringToUuid("300b2aec-a094-43fb-98ff-04917cf7a2fb");

const CCCD = Ble.cccdUuid();

class Boat {
    var connected = false;

    var latitude = 0;
    var longitude = 0;
    var wind_angle = 0.0;
    var wind_speed = 0.0;
    var speed = 0.0;

    function initialize() {
        Ble.setDelegate(new BluetoothDelegate(self));
        Ble.registerProfile({
            :uuid => SERVICE,
            :characteristics => [
                {
                    :uuid => WIND_SCREEN,
                    :descriptors => [CCCD],
                },
            ],
        });
    }
}

class BluetoothDelegate extends Ble.BleDelegate {
    var boat;
    var device = null;
    var descriptor_write as Queue = new Queue();

    function initialize(boat as Boat) {
        BleDelegate.initialize();
        self.boat = boat;
    }

    function onScanResults(scanResults as Ble.Iterator) {
        var scanResult = scanResults.next() as Ble.ScanResult?;
        while (scanResult != null) {
            if (!hasService(scanResult)) {
                scanResult = scanResults.next();
                continue;
            }

            var paired = Ble.pairDevice(scanResult);
            if (paired != null) {
                self.device = paired;
                Ble.setScanState(Ble.SCAN_STATE_OFF);
                return;
            }
        }
    }

    function onProfileRegister(uuid, status) {
        Ble.setScanState(Ble.SCAN_STATE_SCANNING);
    }

    function onConnectedStateChanged(device, state) {
        if (state == Ble.CONNECTION_STATE_CONNECTED) {
            self.device = device;

            var service = device.getService(SERVICE);
            var cccd = service
                .getCharacteristic(WIND_SCREEN)
                .getDescriptor(CCCD);
            self.descriptorWrite(cccd, [2, 0]b);

            WatchUi.switchToView(
                new WindView(self.boat),
                null,
                WatchUi.SLIDE_UP
            );
            return;
        } else if (
            state == Ble.CONNECTION_STATE_DISCONNECTED &&
            device == self.device
        ) {
            self.device = null;
            WatchUi.switchToView(
                new ConnectingView(),
                null,
                WatchUi.SLIDE_DOWN
            );
            Ble.setScanState(Ble.SCAN_STATE_SCANNING);
        }
    }

    function onCharacteristicChanged(characteristic, value) {
        self.handleValue(characteristic, value);
    }

    function onCharacteristicRead(characteristic, status, value) {
        if (status == Ble.STATUS_SUCCESS) {
            self.handleValue(characteristic, value);
        }
    }

    function handleValue(characteristic, value as Lang.ByteArray) {
        var uuid = characteristic.getUuid();
        if (uuid.equals(WIND_SCREEN)) {
            self.boat.speed = u16(value, 0) * 0.01 * MPS_TO_KNOTS;
            self.boat.wind_speed = u16(value, 2) * 0.01 * MPS_TO_KNOTS;
            self.boat.wind_angle = u16(value, 4) * 0.0001;
        }
        WatchUi.requestUpdate();
    }

    function descriptorWrite(descriptor, bytes) {
        if (self.descriptor_write.add([descriptor, bytes])) {
            descriptor.requestWrite(bytes);
        }
    }

    function onDescriptorWrite(descriptor, status) {
        var next = self.descriptor_write.next() as Array?;
        if (next != null) {
            next[0].requestWrite(next[1]);
        }
    }
}

class Queue {
    var in_progress = false;
    var queue as Array = [];

    function add(value as Object) as Boolean {
        if (self.in_progress) {
            self.queue.add(value);
            return false;
        } else {
            self.in_progress = true;
            return true;
        }
    }

    function next() as Object? {
        if (self.queue.size() == 0) {
            self.in_progress = false;
            return null;
        } else {
            var front = self.queue[0] as Array;
            self.queue = self.queue.slice(1, null) as Array;
            return front;
        }
    }
}

function u16(value as Lang.ByteArray, offset as Number) as Number {
    return value.decodeNumber(Lang.NUMBER_FORMAT_UINT16, {
        :offset => offset,
        :endianness => Lang.ENDIAN_LITTLE,
    });
}

function s32(value as Lang.ByteArray, offset as Number) as Number {
    return value.decodeNumber(Lang.NUMBER_FORMAT_SINT32, {
        :offset => offset,
        :endianness => Lang.ENDIAN_LITTLE,
    });
}

function hasService(scanResult as Ble.ScanResult) as Boolean {
    var services = scanResult.getServiceUuids();
    var service = services.next() as Ble.Uuid?;

    while (service != null) {
        if (service.equals(SERVICE)) {
            return true;
        }
        service = services.next();
    }

    return false;
}
