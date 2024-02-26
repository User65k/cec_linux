use cec_linux::*;

fn main() -> std::io::Result<()> {
    let cec = CecDevice::open("/dev/cec0")?;

    cec.set_mode(CecModeInitiator::Send, CecModeFollower::ExclusivePassthru)?;

    let physical_addr = cec.get_phys()?;

    loop {
        let msg = cec.rec()?;
        match msg.opcode() {
            Some(Ok(CecOpcode::ActiveSource)) | Some(Ok(CecOpcode::RoutingInformation)) | Some(Ok(CecOpcode::SetStreamPath))
                if msg.parameters() == physical_addr.to_be_bytes() =>
            {
                // this is not done by the core
                println!("THIS IS US {:?}", msg.opcode().unwrap().unwrap());
                cec.transmit_data(CecLogicalAddress::Playback2, CecLogicalAddress::UnregisteredBroadcast, CecOpcode::ActiveSource, &physical_addr.to_be_bytes())?;
            },
            Some(Ok(CecOpcode::ReportPhysicalAddr)) => {},//core is still taking care of that
            Some(Ok(opcode)) if msg.destination() == CecLogicalAddress::UnregisteredBroadcast => {
                //dont answer brodcasts
                println!("{:?}: {:?} {:x?}", msg.initiator(), opcode, msg.parameters());
            },
            Some(Ok(CecOpcode::GetCecVersion)) => {
                cec.transmit_data(
                    msg.destination(),
                    msg.initiator(),
                    CecOpcode::CecVersion,
                &[
                    Version::V1_3A.into()
                ])?;
            },
            Some(Ok(CecOpcode::GiveDeviceVendorId)) => {
                cec.transmit_data(
                    msg.destination(),
                    msg.initiator(),
                    CecOpcode::FeatureAbort,
                &[
                    CecOpcode::GiveDeviceVendorId.into(),
                    CecAbortReason::Unrecognized.into()
                ])?;/*
                cec.transmit_data(
                    msg.destination(),
                    msg.initiator(),
                    CecOpcode::DeviceVendorId,
                &[0,0,0])?;*/
            },
            Some(Ok(CecOpcode::Abort)) => {
                cec.transmit_data(
                    msg.destination(),
                    msg.initiator(),
                    CecOpcode::FeatureAbort,
                &[
                    CecOpcode::Abort.into(),
                    CecAbortReason::Other.into()
                ])?;
            },
            Some(Ok(CecOpcode::GivePhysicalAddr)) => {
                let l = cec.get_log()?;
                let mut addr = Vec::with_capacity(3);
                
                if let Some(log) = l.addresses().first() {
                    addr.extend_from_slice(&physical_addr.to_be_bytes());
                    addr.push((*log).into());

                    cec.transmit_data(
                        msg.destination(),
                        msg.initiator(),
                        CecOpcode::ReportPhysicalAddr,
                    &addr)?;
                }//else no address yet?!?!?
            },
            Some(Ok(CecOpcode::GiveOsdName)) => {
                cec.transmit_data(
                    msg.destination(),
                    msg.initiator(),
                    CecOpcode::SetOsdName,
                b"pi4")?;
            },
            Some(Ok(CecOpcode::GiveDevicePowerStatus)) => {
                cec.transmit_data(
                    msg.destination(),
                    msg.initiator(),
                    CecOpcode::ReportPowerStatus,
                &[CecPowerStatus::On.into()])?;
            },
            Some(Ok(CecOpcode::GiveFeatures)) => {},
            Some(Ok(opcode)) => {
                println!("{:?} -> {:?} : {:?} {:x?}", msg.initiator(), msg.destination(), opcode, msg.parameters());
            },
            _ => {
                println!("{:?}", msg);
            }
        }
    }
}