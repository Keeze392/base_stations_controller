use std::env::args;
use std::time::Duration;

use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};

use futures::StreamExt;

use tokio::time::timeout;
use uuid::{uuid, Uuid};

// this uuid is for power mode.
const BASE_STATION_UUID: Uuid = uuid!("00001525-1212-efde-1523-785feabcd124");

#[tokio::main]
async fn main() {
    let power_mode: String = match args().nth(1) {
        Some(val) => val,
        None => panic!("missing input, available mode: wake, sleep")
    };

    let manager: Manager = match Manager::new().await {
        Ok(val) => val,
        Err(e) => panic!("Creating Manager failed {}", e)
    };

    // get adapters from manager
    let adapters: Vec<Adapter> = match manager.adapters().await {
        Ok(val) => val,
        Err(e) => panic!("trying get adapters failed {}", e)
    };

    // get device from adapters
    let central: Adapter = match adapters.into_iter().nth(0) {
        Some(val) => val,
        None => panic!("trying get device from adapter failed")
    };

    // start scan to get base stations
    match central.start_scan(ScanFilter::default()).await {
        Ok(()) => {},
        Err(e) => panic!("trying start scanning failed {e}")
    }

    // for get callback from events
    let mut events = match central.events().await {
        Ok(val) => val,
        Err(e) => panic!("trying retrieve event failed {e}")
    };

    // run soon as discover a device, stop the work after 2 secs are up
    timeout(Duration::from_secs(2), async {
        while let Some(event) = events.next().await {
            match event {
                CentralEvent::DeviceDiscovered(id) => {
                    let peripheral = match central.peripheral(&id).await {
                        Ok(val) => val,
                        Err(_) => continue,
                    };

                    let properties = match peripheral.properties().await {
                        Ok(val) => val,
                        Err(_) => continue,
                    };

                    if properties
                        .and_then(|p| p.local_name)
                        .map(|name| name.contains("LHB-"))
                        .unwrap_or(false) {

                        tokio::spawn(send_command(peripheral, power_mode.clone()));
                    }
                }
                _ => {}
            }
        }
    }).await.ok();
}

#[inline]
async fn send_command(p: Peripheral, power_mode: String) {
    match p.connect().await {
        Ok(()) => {},
        Err(e) => eprintln!("Error: trying connect base station but failed {}", e)
    }

    match p.discover_services().await {
        Ok(()) => {},
        Err(e) => eprintln!("Error: trying discover services failed {}", e)
    }

    let chars = p.characteristics();
    let cmd_char: &btleplug::api::Characteristic = match chars.iter().find(|c| c.uuid == BASE_STATION_UUID) {
        Some(t) => t,
        None => panic!("trying find base station uuid failed")
    };

    // send command what to do
    if power_mode.to_lowercase() == "wake" {

        // wake up mode
        match p.write(&cmd_char, &[b'\x01'], WriteType::WithoutResponse).await {
            Ok(()) => {},
            Err(e) => eprintln!("Error: wake up command failed {}", e)
        }
    } else if power_mode.to_lowercase() == "sleep" {

        // sleep mode
        match p.write(&cmd_char, &[b'\x00'], WriteType::WithoutResponse).await {
            Ok(()) => {},
            Err(e) => eprintln!("Error: sleep mode command failed {}", e)
        }
    } else {
        panic!("power mode should name excatly \"wake\" or \"sleep\"");
    }
}
