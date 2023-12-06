#![cfg_attr(docsrs, feature(doc_cfg))]
/*!
 * Use [CEC linux API](https://www.kernel.org/doc/html/v4.9/media/uapi/cec/cec-api.html) in pure rust.
 *
 * Create a [CecDevice] and optionally [change its mode](CecDevice::set_mode)
 * to send and receive messages to and from other devices.
 *
 * ```
 * # fn main() -> std::io::Result<()> {
 * let cec = CecDevice::open("/dev/cec0")?;
 * cec.set_mode(CecModeInitiator::Send, CecModeFollower::All)?;
 * cec.transmit(
 *      CecLogicalAddress::Playback2,
 *      CecLogicalAddress::Audiosystem,
 *      CecOpcode::Standby,
 *  )?;
 * let msg = cec.rec()?;
 * # Ok()
 * # }
 * ```
 */
mod sys;
#[cfg(feature = "poll")]
#[cfg_attr(docsrs, doc(cfg(feature = "poll")))]
pub use nix::poll::PollFlags;
#[cfg(feature = "poll")]
use nix::poll::{poll, PollFd};
use std::{io::Result, os::fd::AsRawFd};
use sys::{
    capabilities, get_event, get_log, get_mode, get_phys, receive, set_log, set_mode, set_phys,
    transmit, CecEvent as CecEventSys, CecEventType, CEC_MODE_FOLLOWER_MSK, CEC_MODE_INITIATOR_MSK,
};
pub use sys::{
    Capabilities, CecAbortReason, CecCaps, CecEventLostMsgs, CecEventStateChange, CecLogAddrFlags,
    CecLogAddrMask, CecLogAddrType, CecLogAddrs, CecLogicalAddress, CecModeFollower,
    CecModeInitiator, CecMsg, CecOpcode, CecPowerStatus, CecPrimDevType, CecTimer,
    CecUserControlCode, DeckControlMode, DeckInfo, DisplayControl, MenuRequestType, OSDStr,
    PlayMode, RecordingSequence, StatusRequest, VendorID, Version, CEC_VENDOR_ID_NONE,
};

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub mod tokio;

/// A handle on a CEC device.
pub struct CecDevice(std::fs::File);

impl CecDevice {
    /**
     * Open a CEC device. Typically `/dev/cecX`
     * ```
     * # fn main() -> std::io::Result<()> {
     * let cec = CecDevice::open("/dev/cec0")?;
     * # Ok()
     * # }
     * ```
     */
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map(Self)
    }
    /// Poll for
    /// 1. newly received Messages (`POLLIN` and `POLLRDNORM` flags)
    /// 2. room in the transmit queue (`POLLOUT` and `POLLWRNORM` flags)
    /// 3. events in the event queue (`POLLPRI` flag)
    ///
    /// timeout is in milliseconds.  
    /// Specifying a negative value in timeout means an infinite timeout.
    /// Specifying a timeout of zero causes poll() to return immediately, even if no file descriptors are ready.
    ///
    /// You might want to look into polling multiple file descriptors at once by using [CecDevice::as_raw_fd] or [tokio::AsyncCec].
    // When the function timed out it returns a value of zero, on failure it returns -1 and the errno variable is set appropriately.
    #[cfg(feature = "poll")]
    #[cfg_attr(docsrs, doc(cfg(feature = "poll")))]
    pub fn poll(&self, events: PollFlags, timeout: i32) -> Result<PollFlags> {
        let mut fds = [PollFd::new(&self.0, events)];
        poll(&mut fds, timeout)?;
        fds[0].revents().ok_or(std::io::ErrorKind::Other.into())
    }
    /// query information on the devices capabilities. See [CecCaps]
    pub fn get_capas(&self) -> Result<CecCaps> {
        let mut capas = CecCaps::default();
        unsafe { capabilities(self.0.as_raw_fd(), &mut capas) }?;
        Ok(capas)
    }
    /// Change this handles mode.
    ///
    /// By default any filehandle can use RECEIVE and TRANSMIT.
    /// This sets initiator and/or follower mode which can be exclusive depending on the chosen mode.  
    /// The initiator is the filehandle that is used to initiate messages, i.e. it commands other CEC devices.  
    /// The follower is the filehandle that receives messages sent to the CEC adapter and processes them.  
    /// The CEC framework will process core messages unless requested otherwise by the follower.
    pub fn set_mode(&self, initiator: CecModeInitiator, follower: CecModeFollower) -> Result<()> {
        let mode = u32::from(initiator) | u32::from(follower);
        unsafe { set_mode(self.0.as_raw_fd(), &mode) }?;
        Ok(())
    }
    pub fn get_mode(&self) -> Result<(CecModeInitiator, CecModeFollower)> {
        let mut mode = 0;
        unsafe { get_mode(self.0.as_raw_fd(), &mut mode) }?;
        let i = CecModeInitiator::try_from(mode & CEC_MODE_INITIATOR_MSK);
        let e = CecModeFollower::try_from(mode & CEC_MODE_FOLLOWER_MSK);
        match (i, e) {
            (Ok(i), Ok(e)) => Ok((i, e)),
            _ => Err(std::io::ErrorKind::Other.into()),
        }
    }
    /**
     * Set the physical address of the adapter.
     *  
     * Only available if [Capabilities::PHYS_ADDR] is set. May not be available if that is handled internally.
     * __Not__ possible with [CecModeInitiator::None].
     *
     * To clear an existing physical address use CEC_PHYS_ADDR_INVALID. The adapter will go to the unconfigured state.  
     * If logical address types have been defined (see [CecDevice::set_log]), then it will block until all requested logical addresses have been claimed. If the file descriptor is in non-blocking mode then it will not wait for the logical addresses to be claimed, instead it just returns.
     *
     * A CEC_EVENT_STATE_CHANGE event is sent when the physical address changes.
     *
     * The physical address is a 16-bit number where each group of 4 bits represent a digit of the physical address a.b.c.d where the most significant 4 bits represent ‘a’. The CEC root device (usually the TV) has address 0.0.0.0. Every device that is hooked up to an input of the TV has address a.0.0.0 (where ‘a’ is ≥ 1), devices hooked up to those in turn have addresses a.b.0.0, etc. So a topology of up to 5 devices deep is supported. The physical address a device shall use is stored in the EDID of the sink.  
     * For example, the EDID for each HDMI input of the TV will have a different physical address of the form a.0.0.0 that the sources will read out and use as their physical address.  
     * If nothing is connected, then phys_addr is 0xffff.
     * See HDMI 1.4b, section 8.7 (Physical Address).
     */
    pub fn set_phys(&self, addr: u16) -> Result<()> {
        unsafe { set_phys(self.0.as_raw_fd(), &addr) }?;
        Ok(())
    }
    /// Query physical addresses
    /// e.g. 0x3300 -> 3.3.0.0
    pub fn get_phys(&self) -> Result<u16> {
        let mut addr = 0;
        unsafe { get_phys(self.0.as_raw_fd(), &mut addr) }?;
        Ok(addr)
    }
    /**
     *  Set logical address.
     *  
     *  Only available if [Capabilities::LOG_ADDRS] is set.
     * __Not__ possible with [CecModeInitiator::None].
     *
     *  To clear existing logical addresses set num_log_addrs to 0. All other fields will be ignored in that case. The adapter will go to the unconfigured state.
     *  Attempting to call set_log when logical address types are already defined will return with error EBUSY.
     *
     *  If the physical address is valid (see [CecDevice::set_phys]), then it will block until all requested logical addresses have been claimed. If the file descriptor is in non-blocking mode then it will not wait for the logical addresses to be claimed, instead it just returns.
     *
     *  A CEC_EVENT_STATE_CHANGE event is sent when the logical addresses are claimed or cleared.
     *
     * */
    pub fn set_log(&self, mut log: CecLogAddrs) -> Result<()> {
        unsafe { set_log(self.0.as_raw_fd(), &mut log) }?;
        Ok(())
    }
    /// Query logical addresses
    pub fn get_log(&self) -> Result<CecLogAddrs> {
        let mut log = CecLogAddrs {
            log_addr: Default::default(),
            log_addr_mask: Default::default(),
            cec_version: Version::V1_4,
            num_log_addrs: 0,
            vendor_id: 0,
            flags: CecLogAddrFlags::empty(),
            osd_name: Default::default(),
            primary_device_type: [CecPrimDevType::TV; 4],
            log_addr_type: [CecLogAddrType::TV; 4],
            all_device_types: [0; 4],
            features: [[0; 4]; 12],
        };
        unsafe { get_log(self.0.as_raw_fd(), &mut log) }?;
        Ok(log)
    }
    pub fn get_event(&self) -> Result<CecEvent> {
        let mut evt = CecEventSys::default();
        unsafe {
            get_event(self.0.as_raw_fd(), &mut evt)?;
            match evt.typ {
                CecEventType::LostMsgs => return Ok(CecEvent::LostMsgs(evt.payload.lost_msgs)),
                CecEventType::StateChange => {
                    return Ok(CecEvent::StateChange(evt.payload.state_change))
                }
            }
        }
        Err(std::io::ErrorKind::Other.into())
    }
    /// wake a remote cec device from standby
    pub fn turn_on(&self, from: CecLogicalAddress, to: CecLogicalAddress) -> Result<()> {
        if to == CecLogicalAddress::Tv {
            self.transmit(from, to, CecOpcode::ImageViewOn)
        } else {
            self.keypress(from, to, CecUserControlCode::Power)
        }
    }
    /// send a button press to a remote cec device
    pub fn keypress(
        &self,
        from: CecLogicalAddress,
        to: CecLogicalAddress,
        key: CecUserControlCode,
    ) -> Result<()> {
        self.transmit_data(from, to, CecOpcode::UserControlPressed, &[key.into()])?;
        self.transmit(from, to, CecOpcode::UserControlReleased)
    }
    /// send a cec command without parameters to a remote device
    pub fn transmit(
        &self,
        from: CecLogicalAddress,
        to: CecLogicalAddress,
        opcode: CecOpcode,
    ) -> Result<()> {
        let mut msg = CecMsg::init(from, to);
        msg.msg[1] = opcode.into();
        msg.len = 2;
        unsafe { transmit(self.0.as_raw_fd(), &mut msg) }?;
        Ok(())
    }
    /// send a cec command with parameters to a remote device.
    /// The format of `data` depends on the `opcode`.
    pub fn transmit_data(
        &self,
        from: CecLogicalAddress,
        to: CecLogicalAddress,
        opcode: CecOpcode,
        data: &[u8],
    ) -> Result<()> {
        let mut msg = CecMsg::init(from, to);
        msg.msg[1] = opcode.into();
        msg.len = 2 + data.len() as u32;
        msg.msg[2..msg.len as usize].copy_from_slice(data);
        unsafe { transmit(self.0.as_raw_fd(), &mut msg) }?;
        Ok(())
    }
    /// receive a single message. the available messages depend on [CecModeFollower]
    pub fn rec(&self) -> Result<CecMsg> {
        let mut msg = CecMsg::init(
            CecLogicalAddress::UnregisteredBroadcast,
            CecLogicalAddress::UnregisteredBroadcast,
        );
        unsafe { receive(self.0.as_raw_fd(), &mut msg) }?;
        Ok(msg)
    }
}

impl AsRawFd for CecDevice {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.0.as_raw_fd()
    }
}

#[derive(Debug)]
pub enum CecEvent {
    /// Event that occurs when the adapter state changes
    StateChange(CecEventStateChange),
    /// This event is sent when messages are lost because the application
    /// didn't empty the message queue in time
    LostMsgs(CecEventLostMsgs),
}
