MCU=esp32c6 cargo espflash save-image --release --chip esp32c6 target/out.bin --target-app-partition ota_0 --partition-table ../partitions.csv
