use std::env::args;
use std::time::Duration;

use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};

use futures::StreamExt;

use tokio::time::timeout;
use uuid::{Uuid, uuid};

// this uuid is for power mode.
const BASE_STATION_UUID: Uuid = uuid!("00001525-1212-efde-1523-785feabcd124");

#[tokio::main]
async fn main() {
    let Some(power_mode): Option<String> = args().nth(1) else {
        panic!("missing input, available mode: wake, sleep")
    };

    let manager: Manager = match Manager::new().await {
        Ok(val) => val,
        Err(e) => panic!("Creating Manager failed {}", e),
    };

    // get adapters from manager
    let adapters: Vec<Adapter> = match manager.adapters().await {
        Ok(val) => val,
        Err(e) => panic!("trying get adapters failed {}", e),
    };

    // get device from adapters
    let Some(central): Option<Adapter> = adapters.into_iter().next() else {
        panic!("trying get device from adapter failed")
    };

    // start scan to get base stations
    if let Err(e) = central.start_scan(ScanFilter::default()).await {
        panic!("trying start scanning failed {e}")
    }

    // for get callback from events
    let mut events = match central.events().await {
        Ok(val) => val,
        Err(e) => panic!("trying retrieve event failed {e}"),
    };

    // run a task soon as discover a device, stop the work after 2 secs are up
    timeout(Duration::from_secs(2), async {
        while let Some(event) = events.next().await {
            if let CentralEvent::DeviceDiscovered(id) = event {
                let Ok(peripheral) = central.peripheral(&id).await else {
                    continue;
                };

                let Ok(properties) = peripheral.properties().await else {
                    continue;
                };

                if properties
                    .and_then(|p| p.local_name)
                    .map(|name| name.contains("LHB-"))
                    .unwrap_or(false)
                {
                    tokio::spawn(send_command(peripheral, power_mode.clone()));
                }
            }
        }
    })
    .await
    .ok();
}

#[inline]
async fn send_command(p: Peripheral, power_mode: String) {
    if let Err(e) = p.connect().await {
        eprintln!("Error: trying connect base station but failed {}", e)
    }

    if let Err(e) = p.discover_services().await {
        eprintln!("Error: trying discover services failed {}", e)
    }

    let chars = p.characteristics();
    let Some(cmd_char): Option<&btleplug::api::Characteristic> =
        chars.iter().find(|c| c.uuid == BASE_STATION_UUID)
    else {
        panic!("trying find base station uuid failed")
    };

    // send command what to do
    if power_mode.to_lowercase() == "wake" {
        // wake up mode
        if let Err(e) = p.write(cmd_char, b"\x01", WriteType::WithResponse).await {
            eprintln!("Error: wake up command failed {}", e)
        }
    } else if power_mode.to_lowercase() == "sleep" {
        // sleep mode
        if let Err(e) = p.write(cmd_char, b"\x00", WriteType::WithResponse).await {
            eprintln!("Error: sleep mode command failed {}", e)
        }
    } else {
        panic!("power mode should name excatly \"wake\" or \"sleep\"");
    }
}

