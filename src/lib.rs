#![cfg_attr(docsrs, feature(doc_cfg))]
/*!
 * Use [CEC linux API](https://www.kernel.org/doc/html/v4.9/media/uapi/cec/cec-api.html) in pure rust.
 *
 * Create a [CecDevice] and optionally [change its mode](CecDevice::set_mode)
 * to send and receive messages to and from other devices.
 *
 * ```no_run
 * # use cec_linux::{CecDevice, CecModeInitiator, CecModeFollower, CecLogicalAddress, CecOpcode};
 * # fn main() -> std::io::Result<()> {
 * let cec = CecDevice::open("/dev/cec0")?;
 * cec.set_mode(CecModeInitiator::Send, CecModeFollower::All)?;
 * cec.transmit(
 *      CecLogicalAddress::Playback2,
 *      CecLogicalAddress::Audiosystem,
 *      CecOpcode::Standby,
 *  )?;
 * let msg = cec.rec()?;
 * # Ok(())
 * # }
 * ```
 */
mod sys;
#[cfg(feature = "poll")]
use nix::poll::{poll, PollFd};
#[cfg(feature = "poll")]
#[cfg_attr(docsrs, doc(cfg(feature = "poll")))]
pub use nix::poll::{PollFlags, PollTimeout};
use std::{
    io::Result,
    mem::MaybeUninit,
    os::fd::{AsFd, AsRawFd},
};
use sys::{
    capabilities, get_event, get_log, get_mode, get_phys, receive, set_log, set_mode, set_phys,
    transmit, CecEventType, CecTxError, RxStatus, TxStatus, CEC_MODE_FOLLOWER_MSK,
    CEC_MODE_INITIATOR_MSK,
};
pub use sys::{
    Capabilities, CecAbortReason, CecCaps, CecEventLostMsgs, CecEventStateChange, CecLogAddrFlags,
    CecLogAddrMask, CecLogAddrType, CecLogAddrs, CecLogicalAddress, CecModeFollower,
    CecModeInitiator, CecMsg, CecOpcode, CecPhysicalAddress, CecPowerStatus, CecPrimDevType,
    CecTimer, CecUserControlCode, DeckControlMode, DeckInfo, DisplayControl, MenuRequestType,
    OSDStr, PlayMode, RecordingSequence, StatusRequest, VendorID, Version,
};

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub mod tokio;

/// A handle on a CEC device.
pub struct CecDevice(std::fs::File);

impl CecDevice {
    /**
     * Open a CEC device. Typically `/dev/cecX`
     * ```no_run
     * # use cec_linux::CecDevice;
     * # fn main() -> std::io::Result<()> {
     * let cec = CecDevice::open("/dev/cec0")?;
     * # Ok(())
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
    /// Specifying a [`PollTimeout::NONE`] in timeout means an infinite timeout.
    /// Specifying a timeout of [`PollTimeout::ZERO`] causes poll() to return immediately, even if no file descriptors are ready.
    ///
    /// You might want to look into polling multiple file descriptors at once by using [CecDevice::as_raw_fd] or [tokio::AsyncCec].
    // When the function timed out it returns a value of zero, on failure it returns -1 and the errno variable is set appropriately.
    #[cfg(feature = "poll")]
    #[cfg_attr(docsrs, doc(cfg(feature = "poll")))]
    pub fn poll<T: Into<PollTimeout>>(&self, events: PollFlags, timeout: T) -> Result<PollFlags> {
        let mut fds = [PollFd::new(self.0.as_fd(), events)];
        poll(&mut fds, timeout)?;
        fds[0].revents().ok_or(std::io::ErrorKind::Other.into())
    }
    /// query information on the devices capabilities. See [CecCaps]
    pub fn get_capas(&self) -> Result<CecCaps> {
        let mut capas = MaybeUninit::uninit();
        unsafe { capabilities(self.0.as_raw_fd(), capas.as_mut_ptr()) }?;
        Ok(unsafe { capas.assume_init() })
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
     * To clear an existing physical address use [CecPhysicalAddress::INVALID]. The adapter will go to the unconfigured state.  
     * If logical address types have been defined (see [CecDevice::set_log]), then it will block until all requested logical addresses have been claimed. If the file descriptor is in non-blocking mode then it will not wait for the logical addresses to be claimed, instead it just returns.
     *
     * A [CecEvent::StateChange] event is sent when the physical address changes.
     */
    pub fn set_phys(&self, addr: CecPhysicalAddress) -> Result<()> {
        unsafe { set_phys(self.0.as_raw_fd(), &addr) }?;
        Ok(())
    }
    /// Query physical addresses
    /// If nothing is connected, then phys_addr is [CecPhysicalAddress::INVALID].
    pub fn get_phys(&self) -> Result<CecPhysicalAddress> {
        let mut addr = CecPhysicalAddress::INVALID;
        unsafe { get_phys(self.0.as_raw_fd(), &mut addr) }?;
        Ok(addr)
    }
    /**
     *  Set logical address.
     *  
     *  Only available if [Capabilities::LOG_ADDRS] is set.
     * __Not__ possible with [CecModeInitiator::None].
     *
     *  To clear existing logical addresses set [CecLogAddrs::default()]. The adapter will go to the unconfigured state.
     *  Attempting to call set_log when logical address types are already defined will return with error EBUSY.
     *
     *  If the physical address is valid (see [CecDevice::set_phys]), then it will block until all requested logical addresses have been claimed. If the file descriptor is in non-blocking mode then it will not wait for the logical addresses to be claimed, instead it just returns.
     *
     *  A [CecEvent::StateChange] event is sent when the logical addresses are claimed or cleared.
     *
     * */
    pub fn set_log(&self, mut log: CecLogAddrs) -> Result<()> {
        unsafe { set_log(self.0.as_raw_fd(), &mut log) }?;
        Ok(())
    }
    /// Query logical addresses
    pub fn get_log(&self) -> Result<CecLogAddrs> {
        let mut log = MaybeUninit::uninit();
        unsafe { get_log(self.0.as_raw_fd(), log.as_mut_ptr()) }?;
        Ok(unsafe { log.assume_init() })
    }
    pub fn get_event(&self) -> Result<CecEvent> {
        let mut evt = MaybeUninit::uninit();
        unsafe {
            get_event(self.0.as_raw_fd(), evt.as_mut_ptr())?;
            let evt = evt.assume_init();
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
    ///
    /// transmitting from an address not in [CecLogAddrMask] will return InvalidInput
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
        msg_to_io_result(msg)
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
        msg_to_io_result(msg)
    }
    /**
     * send a cec command with parameters and wait for a reply with opcode `wait_for`. Then return its payload.
     * returns timeout if no reply is received
     * ```no_run
     * # use cec_linux::{CecDevice, CecLogicalAddress, CecOpcode};
     * # fn main() -> std::io::Result<()> {
     * # let cec = CecDevice::open("/dev/cec0")?;
     * if let Ok(audio) = cec.request_data(CecLogicalAddress::Playback2, CecLogicalAddress::Audiosystem, CecOpcode::GiveAudioStatus, b"", CecOpcode::ReportAudioStatus){
     *    let v = audio[0];
     *    println!("Muted: {}", v & 0x80);
     *    println!("Vol: {}%", v & 0x7f);
     * }
     * # Ok(())
     * # }
     * ```
     */
    pub fn request_data(
        &self,
        from: CecLogicalAddress,
        to: CecLogicalAddress,
        opcode: CecOpcode,
        data: &[u8],
        wait_for: CecOpcode,
    ) -> Result<Vec<u8>> {
        let mut msg = CecMsg::init(from, to);
        msg.msg[1] = opcode.into();
        msg.len = 2 + data.len() as u32;
        msg.msg[2..msg.len as usize].copy_from_slice(data);
        msg.reply = wait_for;
        msg.timeout = 1000;
        unsafe { transmit(self.0.as_raw_fd(), &mut msg) }?;
        if msg.reply == CecOpcode::FeatureAbort && !msg.tx_status.contains(TxStatus::OK) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                CecTxError::from(msg),
            ));
        }
        if msg.reply != CecOpcode::FeatureAbort
            || (msg.reply == CecOpcode::FeatureAbort
                && msg.rx_status.contains(RxStatus::FEATURE_ABORT))
        {
            let l = msg.len as usize;
            let data = if l > 2 {
                let mut data = Vec::with_capacity(l - 2);
                data.extend_from_slice(&msg.msg[2..l]);
                data
            } else {
                Vec::with_capacity(0)
            };
            return Ok(data);
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            CecTxError::from(msg),
        ))
    }
    /// receive a single message.
    /// block forever
    /// the available messages depend on [CecModeFollower]
    #[inline]
    pub fn rec(&self) -> Result<CecMsg> {
        self.rec_for(0)
    }
    /// receive a single message.
    /// block for at most `timeout` ms.
    /// the available messages depend on [CecModeFollower]
    pub fn rec_for(&self, timeout: u32) -> Result<CecMsg> {
        let mut msg = MaybeUninit::uninit();
        let ptr: *mut CecMsg = msg.as_mut_ptr();
        unsafe { std::ptr::addr_of_mut!((*ptr).timeout).write(timeout) };
        unsafe { receive(self.0.as_raw_fd(), ptr) }?;
        Ok(unsafe { msg.assume_init() })
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

/// Turn a message into io::Result
fn msg_to_io_result(msg: CecMsg) -> Result<()> {
    if msg.tx_status.contains(TxStatus::OK) {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            CecTxError::from(msg),
        ))
    }
}
