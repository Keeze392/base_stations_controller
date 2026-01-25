use std::time::Duration;
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use tokio::time;
use uuid::{uuid, Uuid};
use std::env::args;

#[tokio::main]
async fn main() {
    let power_mode = args().nth(1).expect("missing input, available mode: wake, sleep");

    // this uuid is for power mode.
    const BASE_STATION_UUID: Uuid = uuid!("00001525-1212-efde-1523-785feabcd124");

    // get ble adapter and start
    let manager = Manager::new().await.unwrap();
    let adapters = manager.adapters().await.unwrap();
    let central = adapters.into_iter().nth(0).unwrap();

    // start scan to get base stations
    central.start_scan(ScanFilter::default()).await.unwrap();
    time::sleep(Duration::from_secs(3)).await;
    
    let base_stations = find_base_stations(&central).await.unwrap();

    // connect it and get service
    for p in base_stations {
        p.connect().await.unwrap();
        p.discover_services().await.unwrap();

        // if match
        let chars = p.characteristics();
        let cmd_char = chars.iter().find(|c| c.uuid == BASE_STATION_UUID).unwrap();

        // send command what to do
        if power_mode.to_lowercase() == "wake" {
            // wake up mode
            p.write(&cmd_char, &[b'\x01'], WriteType::WithoutResponse).await.unwrap();

        } else if power_mode.to_lowercase() == "sleep" {
            // sleep mode
            p.write(&cmd_char, &[b'\x00'], WriteType::WithoutResponse).await.unwrap();
        } else {
            panic!("power mode should name excatly \"wake\" or \"sleep\"");
        }
    }
}

async fn find_base_stations(central: &Adapter) -> Option<Vec<Peripheral>> {
    let mut base_stations: Vec<Peripheral> = Vec::new();

    for p in central.peripherals().await.unwrap() {
        if p.properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .iter()
            .any(|name| name.contains("LHB-"))
        {
            base_stations.push(p);
        }
    }

    Some(base_stations)
}
