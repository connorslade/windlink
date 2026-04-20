using Toybox.BluetoothLowEnergy as Ble;
import Toybox.Lang;
import Toybox.System;
import Toybox.WatchUi;

const DEVICE_NAME = "windlink";
const SERVICE = Ble.stringToUuid("76e20500-da73-4971-bb03-6105e39db3d6");
const POSITION = Ble.stringToUuid("300b2aec-a094-43fb-98ff-04917cf7a2fb");
const SPEED = Ble.stringToUuid("d948b9e5-6626-4d41-8967-c4dca26db1fd");
const WIND = Ble.stringToUuid("91331b1c-3132-4197-aa43-b86b7df421f1");

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
                    :uuid => POSITION,
                    :descriptors => [CCCD],
                },
                {
                    :uuid => SPEED,
                    :descriptors => [CCCD],
                },
                {
                    :uuid => WIND,
                    :descriptors => [CCCD],
                },
            ],
        });
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

    function next() as Object | Null {
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

class BluetoothDelegate extends Ble.BleDelegate {
    var boat;
    var device = null;
    var descriptor_write as Queue = new Queue();

    function descriptorWrite(descriptor, bytes) {
        if (self.descriptor_write.add([descriptor, bytes])) {
            descriptor.requestWrite(bytes);
        }
    }

    function onDescriptorWrite(descriptor, status) {
        var next = self.descriptor_write.next() as Array | Null;
        if (next != null) {
            next[0].requestWrite(next[1]);
        }
    }

    //

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

            var CHARS = [POSITION, SPEED, WIND];
            var service = device.getService(SERVICE);
            for (var i = 0; i < CHARS.size(); i++) {
                var cccd = service
                    .getCharacteristic(CHARS[i])
                    .getDescriptor(CCCD);
                self.descriptorWrite(cccd, [1, 0]b);
            }

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
        System.println("Got CHAR: " + uuid.toString());
        if (uuid.equals(POSITION)) {
            self.boat.longitude = s32(value, 0);
            self.boat.latitude = s32(value, 4);
        } else if (uuid.equals(SPEED)) {
            System.println(" \\ Got Speed: " + value.toString());
            self.boat.speed = u16(value, 0) * 0.01 * MPS_TO_KNOTS;
        } else if (uuid.equals(WIND)) {
            System.println(" \\ Got Wind: " + value.toString());
            System.println("   - speed" + u16(value, 0).toString());
            System.println("   - angle" + u16(value, 2).toString());
            self.boat.wind_speed = u16(value, 0) * 0.01 * MPS_TO_KNOTS;
            self.boat.wind_angle = u16(value, 2) * 0.0001;
        }
        WatchUi.requestUpdate();
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
