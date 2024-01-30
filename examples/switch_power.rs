/*!
 * Set Logic address and switch Devices from and to standby
 */

 use std::{thread::sleep, time::Duration};

use cec_linux::*;

fn main() -> std::io::Result<()> {
    let cec = CecDevice::open("/dev/cec0")?;
    let capas = cec.get_capas()?;
    println!("capas  {:?}", capas);

    // clear existing logical addresses
    let log = CecLogAddrs::default();
    cec.set_log(log)?;

    // set new address (PLAYBACK)
    let log = CecLogAddrs {
        cec_version: Version::V1_4,
        num_log_addrs: 1,
        vendor_id: CEC_VENDOR_ID_NONE,
        osd_name: "pi4".to_string().try_into().unwrap(),
        primary_device_type: [CecPrimDevType::PLAYBACK; 4],
        log_addr_type: [CecLogAddrType::PLAYBACK; 4],
        ..Default::default()
    };
    cec.set_log(log)?;

    // ask Audiosystem to turn on (from standby)
    cec.turn_on(CecLogicalAddress::Playback2, CecLogicalAddress::Audiosystem)?;

    sleep(Duration::from_millis(10000));

    // ask Audiosystem to switch to standby
    cec.transmit(
        CecLogicalAddress::Playback2,
        CecLogicalAddress::Audiosystem,
        CecOpcode::Standby,
    )?;

    sleep(Duration::from_millis(10000));

    // ask TV to turn on
    cec.turn_on(CecLogicalAddress::Playback2, CecLogicalAddress::Tv)?;

    sleep(Duration::from_millis(10000));

    // ask TV to switch to standby
    cec.transmit(
        CecLogicalAddress::Playback2,
        CecLogicalAddress::Tv,
        CecOpcode::Standby,
    )?;

    Ok(())
}
