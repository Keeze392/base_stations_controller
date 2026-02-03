use std::time::Duration;
use std::env::args;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};

use tokio::time;

use uuid::{uuid, Uuid};

#[tokio::main]
async fn main() {
    let power_mode: String = match args().nth(1) {
        Some(v) => v,
        None => panic!("missing input, available mode: wake, sleep")
    };

    // this uuid is for power mode.
    const BASE_STATION_UUID: Uuid = uuid!("00001525-1212-efde-1523-785feabcd124");

    let manager: Manager = match Manager::new().await {
        Ok(m) => m,
        Err(e) => panic!("Creating Manager failed {}", e)
    };

    // get adapters from manager
    let adapters: Vec<Adapter> = match manager.adapters().await {
        Ok(a) => a,
        Err(e) => panic!("trying get adapters failed {}", e)
    };

    // get device from adapters
    let central: Adapter = match adapters.into_iter().nth(0) {
        Some(c) => c,
        None => panic!("trying get device from adapter failed")
    };

    // start scan to get base stations
    match central.start_scan(ScanFilter::default()).await {
        Ok(()) => {},
        Err(e) => panic!("trying start scanning failed {}", e)
    }

    time::sleep(Duration::from_secs(4)).await;
    
    let base_stations: Vec<Peripheral> = match find_base_stations(&central).await {
        Ok(b) => b,
        Err(e) => panic!("No base station detected {}", e)
    };

    // connect it and get service
    for p in base_stations {
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
}

async fn find_base_stations(central: &Adapter) -> Result<Vec<Peripheral>, btleplug::Error> {
    let mut base_stations: Vec<Peripheral> = Vec::new();

    for p in central.peripherals().await? {
        if let Some(name) = p.properties().await? {
            if name.local_name.iter().any(|name| name.contains("LHB-")) {
                println!("Found base station device: {}", name.local_name.as_deref().unwrap_or("unknown"));
                base_stations.push(p);
            }
        }
    }

    Ok(base_stations)
}
