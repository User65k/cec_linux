//https://www.avsforum.com/attachments/hdmi-cec-v1-3a-specifications-pdf.2579760/

use bitflags::bitflags;
use nix::{ioctl_read, ioctl_readwrite, ioctl_write_ptr};
use num_enum::{IntoPrimitive, TryFromPrimitive, TryFromPrimitiveError};

//#define CEC_ADAP_G_CAPS         _IOWR('a',  0, struct cec_caps)
ioctl_readwrite! {
    /// Query device capabilities
    /// Filled by the driver.
    capabilities, b'a',  0, CecCaps
}

/// information about the CEC adapter

#[derive(Debug)]
#[repr(C)]
pub struct CecCaps {
    /// name of the CEC device driver
    driver: OSDStr<32>,
    /// name of the CEC device. @driver + @name must be unique
    name: OSDStr<32>,
    /// number of available logical addresses
    available_log_addrs: u32,
    /// capabilities of the CEC adapter
    capabilities: Capabilities,
    /// version of the CEC adapter framework
    version: u32,
}
impl CecCaps {
    /// number of available logical addresses
    #[inline]
    pub fn available_log_addrs(&self) -> u32 {
        self.available_log_addrs
    }
    /// capabilities of the CEC adapter
    #[inline]
    pub fn capabilities(&self) -> Capabilities {
        self.capabilities
    }
}
impl Default for CecCaps {
    fn default() -> Self {
        Self {
            driver: Default::default(),
            name: Default::default(),
            available_log_addrs: Default::default(),
            capabilities: Capabilities::empty(),
            version: Default::default(),
        }
    }
}

bitflags! {
    /// capabilities of the CEC adapter
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Capabilities: u32 {
        /// Userspace has to configure the physical address. Do so via [CecDevice::set_phys](super::CecDevice::set_phys)
        const PHYS_ADDR = 0b00000001;
        /// Userspace has to configure the logical addresses. Do so via [CecDevice::set_log](super::CecDevice::set_log)
        const LOG_ADDRS = 0b00000010;
        /// Userspace can transmit messages (and thus become [follower](CecModeFollower) as well)
        const TRANSMIT = 0b00000100;

        /// Passthrough all messages instead of processing them.
        const PASSTHROUGH = 0b00001000;
        /// Supports remote control
        const RC = 0b00010000;
        /// Hardware can monitor all messages, not just directed and broadcast.
        /// Needed for [CecModeFollower::MonitorAll]
        const MONITOR_ALL = 0b00100000;
    }
}

// CEC_ADAP_S_LOG_ADDRS
ioctl_readwrite! {
    /// The ioctl CEC_ADAP_S_LOG_ADDRS is only available if CEC_CAP_LOG_ADDRS is set (the ENOTTY error code is returned otherwise). The ioctl CEC_ADAP_S_LOG_ADDRS can only be called by a file descriptor in initiator mode (see ioctls CEC_G_MODE and CEC_S_MODE), if not the EBUSY error code will be returned.
    /// To clear existing logical addresses set num_log_addrs to 0. All other fields will be ignored in that case. The adapter will go to the unconfigured state.
    /// If the physical address is valid (see ioctl CEC_ADAP_S_PHYS_ADDR), then this ioctl will block until all requested logical addresses have been claimed. If the file descriptor is in non-blocking mode then it will not wait for the logical addresses to be claimed, instead it just returns 0.
    /// A CEC_EVENT_STATE_CHANGE event is sent when the logical addresses are claimed or cleared.
    /// Attempting to call ioctl CEC_ADAP_S_LOG_ADDRS when logical address types are already defined will return with error EBUSY.
    set_log, b'a',  4, CecLogAddrs
}

// CEC_ADAP_G_LOG_ADDRS
ioctl_read! {
    /// Query logical addresses
    /// Filled by the driver.
    get_log, b'a',  3, CecLogAddrs
}

/// CEC logical addresses structure
#[derive(Debug)]
#[repr(C)]
pub struct CecLogAddrs {
    /// the claimed logical addresses. Set by the driver.
    pub log_addr: [u8; CEC_MAX_LOG_ADDRS],
    /// current logical address mask. Set by the driver.
    pub log_addr_mask: CecLogAddrMask,

    /// the CEC version that the adapter should implement. Set by the caller.
    /// Used to implement the [CecOpcode::CecVersion] and [CecOpcode::ReportFeatures] messages.
    pub cec_version: Version,
    /// how many logical addresses should be claimed. Set by the caller.
    ///
    /// Must be ≤ [CecCaps::available_log_addrs].
    /// All arrays in this structure are only filled up to index available_log_addrs-1. The remaining array elements will be ignored.
    ///
    /// Note that the CEC 2.0 standard allows for a maximum of 2 logical addresses, although some hardware has support for more. CEC_MAX_LOG_ADDRS is 4.
    ///
    /// The driver will return the actual number of logical addresses it could claim, which may be less than what was requested.
    ///
    /// If this field is set to 0, then the CEC adapter shall clear all claimed logical addresses and all other fields will be ignored.
    pub num_log_addrs: u8,
    /// the vendor ID of the device. Set by the caller.
    pub vendor_id: u32,
    pub flags: CecLogAddrFlags,
    /// the OSD name of the device. Set by the caller
    /// Used for [CecOpcode::SetOsdName]
    pub osd_name: OSDStr<15>,
    /// the primary device type for each logical address. Set by the caller.
    pub primary_device_type: [CecPrimDevType; CEC_MAX_LOG_ADDRS],
    /// the logical address types. Set by the caller.
    pub log_addr_type: [CecLogAddrType; CEC_MAX_LOG_ADDRS],

    /// CEC 2.0: all device types represented by the logical address. Set by the caller. Used in [CecOpcode::ReportFeatures].
    pub all_device_types: [u8; CEC_MAX_LOG_ADDRS],
    /// CEC 2.0: The logical address features. Set by the caller. Used in [CecOpcode::ReportFeatures].
    pub features: [[u8; CEC_MAX_LOG_ADDRS]; 12],
}
impl Default for CecLogAddrs {
    fn default() -> Self {
        Self {
            log_addr: Default::default(),
            log_addr_mask: Default::default(),
            cec_version: Version::V1_4,
            num_log_addrs: 0,
            vendor_id: Default::default(),
            flags: CecLogAddrFlags::empty(),
            osd_name: Default::default(),
            primary_device_type: [CecPrimDevType::PLAYBACK; 4],
            log_addr_type: [CecLogAddrType::PLAYBACK; 4],
            all_device_types: Default::default(),
            features: Default::default(),
        }
    }
}

bitflags! {
    /// Flags for [CecLogAddrs]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CecLogAddrFlags : u32 {
        /// By default if no logical address of the requested type can be claimed, then it will go back to the unconfigured state. If this flag is set, then it will fallback to the Unregistered logical address. Note that if the Unregistered logical address was explicitly requested, then this flag has no effect.
        const ALLOW_UNREG_FALLBACK = (1 << 0);
    }
}
/// CEC Version Operand for [CecOpcode::CecVersion]
#[repr(u8)]
#[non_exhaustive]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Copy, Clone)]
pub enum Version {
    V1_3A = 4,
    V1_4 = 5,
    V2_0 = 6,
}

/// Primary Device Type Operand (prim_devtype)
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Copy, Clone)]
#[repr(u8)]
pub enum CecPrimDevType {
    TV = 0,
    RECORD = 1,
    TUNER = 3,
    PLAYBACK = 4,
    AUDIOSYSTEM = 5,
    SWITCH = 6,
    PROCESSOR = 7,
}
/// The logical address types that the CEC device wants to claim
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Copy, Clone)]
#[repr(u8)]
pub enum CecLogAddrType {
    TV = 0,
    RECORD = 1,
    TUNER = 2,
    PLAYBACK = 3,
    AUDIOSYSTEM = 4,
    SPECIFIC = 5,
    UNREGISTERED = 6,
}
/*// All Device Types Operand (all_device_types)
const CEC_OP_ALL_DEVTYPE_TV: u8 = 0x80;
const CEC_OP_ALL_DEVTYPE_RECORD: u8 = 0x40;
const CEC_OP_ALL_DEVTYPE_TUNER: u8 = 0x20;
const CEC_OP_ALL_DEVTYPE_PLAYBACK: u8 = 0x10;
const CEC_OP_ALL_DEVTYPE_AUDIOSYSTEM: u8 = 0x08;
const CEC_OP_ALL_DEVTYPE_SWITCH: u8 = 0x04;
 */

//#define CEC_ADAP_G_PHYS_ADDR    _IOR('a',  1, __u16)
ioctl_read! {
    /// Query physical addresses
    /// Filled by the driver.
    get_phys, b'a',  1, u16
}

/*
 * phys_addr is either 0 (if this is the CEC root device)
 * or a valid physical address obtained from the sink's EDID
 * as read by this CEC device (if this is a source device)
 * or a physical address obtained and modified from a sink
 * EDID and used for a sink CEC device.
 * If nothing is connected, then phys_addr is 0xffff.
 * See HDMI 1.4b, section 8.7 (Physical Address).
 *
 * The CEC_ADAP_S_PHYS_ADDR ioctl may not be available if that is handled
 * internally.
 */
//#define CEC_ADAP_S_PHYS_ADDR    _IOW('a',  2, __u16)
ioctl_write_ptr! {
    /// The ioctl CEC_ADAP_S_PHYS_ADDR is only available if CEC_CAP_PHYS_ADDR is set (the ENOTTY error code will be returned otherwise). The ioctl CEC_ADAP_S_PHYS_ADDR can only be called by a file descriptor in initiator mode (see ioctls CEC_G_MODE and CEC_S_MODE), if not the EBUSY error code will be returned.
    /// To clear an existing physical address use CEC_PHYS_ADDR_INVALID. The adapter will go to the unconfigured state.
    /// If logical address types have been defined (see ioctl CEC_ADAP_S_LOG_ADDRS), then this ioctl will block until all requested logical addresses have been claimed. If the file descriptor is in non-blocking mode then it will not wait for the logical addresses to be claimed, instead it just returns 0.
    /// A CEC_EVENT_STATE_CHANGE event is sent when the physical address changes.
    /// The physical address is a 16-bit number where each group of 4 bits represent a digit of the physical address a.b.c.d where the most significant 4 bits represent ‘a’. The CEC root device (usually the TV) has address 0.0.0.0. Every device that is hooked up to an input of the TV has address a.0.0.0 (where ‘a’ is ≥ 1), devices hooked up to those in turn have addresses a.b.0.0, etc. So a topology of up to 5 devices deep is supported. The physical address a device shall use is stored in the EDID of the sink.
    /// For example, the EDID for each HDMI input of the TV will have a different physical address of the form a.0.0.0 that the sources will read out and use as their physical address.
    set_phys, b'a',  2, u16
}

//#define CEC_G_MODE              _IOR('a',  8, __u32)
ioctl_read! {
    /// Query mode
    /// Filled by the driver.
    get_mode, b'a',  8, u32
}
//#define CEC_S_MODE              _IOW('a',  9, __u32)
ioctl_write_ptr! {
    /// When a CEC message is received, then the CEC framework will decide how it will be processed.
    /// If the message is a reply to an earlier transmitted message, then the reply is sent back to the filehandle that is waiting for it. In addition the CEC framework will process it.
    /// If the message is not a reply, then the CEC framework will process it first.
    /// If there is no follower, then the message is just discarded and a feature abort is sent back to the initiator if the framework couldn’t process it.
    /// The framework expects the follower to make the right decisions.
    /// See Core Message Processing for details.
    /// If there is no initiator, then any CEC filehandle can use ioctl CEC_TRANSMIT.
    /// If there is an exclusive initiator then only that initiator can call ioctls CEC_RECEIVE and CEC_TRANSMIT.
    /// The follower can of course always call ioctl CEC_TRANSMIT.
    set_mode, b'a',  9, u32
}
// ---  The message handling modes  ---
/// Modes for initiator
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Copy)]
#[repr(u32)]
pub enum CecModeInitiator {
    /// Transmiting not possible (but others can)
    None = 0,
    /// **Default** Shared access
    Send = 1,
    /// Do not allow other senders
    Exclusive = 2,
}
pub const CEC_MODE_INITIATOR_MSK: u32 = 0x0f;
/// Modes for follower
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Copy)]
#[repr(u32)]
pub enum CecModeFollower {
    /// **Default**: Only retrieve replies to own (this handles) messages
    RepliesOnly = 0x0 << 4,
    /// Retrieve all messages for this device.  
    /// __Not__ possible with [CecModeInitiator::None]. Needs [Capabilities::TRANSMIT].
    All = 0x1 << 4,
    /// Retrieve all messages and lock this device.  
    /// __Not__ possible with [CecModeInitiator::None]. Needs [Capabilities::TRANSMIT].
    Exclusive = 0x2 << 4,
    /// Passthrough mode. The CEC framework will pass on most core messages without processing them and the follower will have to implement those messages.
    /// There are some messages that the core will always process, regardless of the passthrough mode.  
    /// __Not__ possible with [CecModeInitiator::None]. Needs [Capabilities::TRANSMIT].
    ///
    /// Core messgages:
    ///  - [CecOpcode::GetCecVersion]
    ///  - [CecOpcode::GiveDeviceVendorId]
    ///  - [CecOpcode::Abort]
    ///  - [CecOpcode::GivePhysicalAddr]
    ///  - [CecOpcode::GiveOsdName]
    ///  - [CecOpcode::GiveFeatures]
    ///  - [CecOpcode::UserControlPressed]
    ///  - [CecOpcode::UserControlReleased]
    ///  - [CecOpcode::ReportPhysicalAddr]
    ExclusivePassthru = 0x3 << 4,
    /// Get all messages sent or received (directed or brodcasted) by this device.
    /// Only possible with [CecModeInitiator::None]. Needs `CAP_NET_ADMIN`.
    Monitor = 0xe << 4,
    /// As above but for all messages on the bus.  
    /// Additionally needs [Capabilities::MONITOR_ALL].
    MonitorAll = 0xf << 4,
}
pub const CEC_MODE_FOLLOWER_MSK: u32 = 0xf0;
// ---  Transmit/receive a CEC command  ---
//#define CEC_TRANSMIT            _IOWR('a',  5, struct cec_msg)
ioctl_readwrite! {
    /// To send a CEC message the application has to fill in the struct :c:type:` cec_msg` and pass it to ioctl CEC_TRANSMIT. The ioctl CEC_TRANSMIT is only available if CEC_CAP_TRANSMIT is set. If there is no more room in the transmit queue, then it will return -1 and set errno to the EBUSY error code. The transmit queue has enough room for 18 messages (about 1 second worth of 2-byte messages). Note that the CEC kernel framework will also reply to core messages (see :ref:cec-core-processing), so it is not a good idea to fully fill up the transmit queue.
    /// If the file descriptor is in non-blocking mode then the transmit will return 0 and the result of the transmit will be available via ioctl CEC_RECEIVE once the transmit has finished (including waiting for a reply, if requested).
    /// The sequence field is filled in for every transmit and this can be checked against the received messages to find the corresponding transmit result.
    transmit, b'a',  5, CecMsg
}
//#define CEC_RECEIVE             _IOWR('a',  6, struct cec_msg)
ioctl_readwrite! {
    /// To receive a CEC message the application has to fill in the timeout field of struct cec_msg and pass it to ioctl CEC_RECEIVE. If the file descriptor is in non-blocking mode and there are no received messages pending, then it will return -1 and set errno to the EAGAIN error code. If the file descriptor is in blocking mode and timeout is non-zero and no message arrived within timeout milliseconds, then it will return -1 and set errno to the ETIMEDOUT error code.
    /// A received message can be:
    /// - a message received from another CEC device (the sequence field will be 0).
    /// - the result of an earlier non-blocking transmit (the sequence field will be non-zero).
    receive, b'a',  6, CecMsg
}

const CEC_MAX_MSG_SIZE: usize = 16;

#[derive(Debug)]
#[repr(C)]
pub struct CecMsg {
    /// Timestamp in nanoseconds using CLOCK_MONOTONIC. Set by the driver when the message transmission has finished.
    tx_ts: u64,
    /// Timestamp in nanoseconds using CLOCK_MONOTONIC. Set by the driver when the message was received.
    rx_ts: u64,
    /// Length in bytes of the message.
    pub len: u32,
    /// The timeout (in ms) that is used to timeout CEC_RECEIVE.
    /// Set to 0 if you want to wait forever. This timeout can also be
    /// used with CEC_TRANSMIT as the timeout for waiting for a reply.
    /// If 0, then it will use a 1 second timeout instead of waiting
    /// forever as is done with CEC_RECEIVE.
    pub timeout: u32,
    /// The framework assigns a sequence number to messages that are sent. This can be used to track replies to previously sent messages.
    pub sequence: u32,
    /// No flags are defined yet, so set this to 0.
    flags: u32,
    /// The message payload.  
    /// Includes initiator, destination and opcode.
    pub msg: [u8; CEC_MAX_MSG_SIZE],
    /// This field is ignored with CEC_RECEIVE and is only used by CEC_TRANSMIT.
    /// If non-zero, then wait for a reply with this opcode.
    /// Set to CEC_MSG_FEATURE_ABORT if you want to wait for a possible ABORT reply.
    ///
    /// If there was an error when sending the
    /// msg or FeatureAbort was returned, then reply is set to 0.
    ///
    /// If reply is non-zero upon return, then len/msg are set to
    /// the received message.
    /// If reply is zero upon return and status has the
    /// CEC_TX_STATUS_FEATURE_ABORT bit set, then len/msg are set to
    /// the received feature abort message.
    /// If reply is zero upon return and status has the
    /// CEC_TX_STATUS_MAX_RETRIES bit set, then no reply was seen at all.
    /// If reply is non-zero for CEC_TRANSMIT and the message is a
    /// broadcast, then -EINVAL is returned.
    /// if reply is non-zero, then timeout is set to 1000 (the required
    /// maximum response time).
    reply: u8,
    /// The message receive status bits. Set by the driver.
    rx_status: RxStatus,
    /// The message transmit status bits. Set by the driver.
    tx_status: TxStatus,
    /// The number of 'Arbitration Lost' events. Set by the driver.
    tx_arb_lost_cnt: u8,
    /// The number of 'Not Acknowledged' events. Set by the driver.
    tx_nack_cnt: u8,
    /// The number of 'Low Drive Detected' events. Set by the driver.
    tx_low_drive_cnt: u8,
    /// The number of 'Error' events. Set by the driver.
    tx_error_cnt: u8,
}
impl CecMsg {
    /// return the initiator's logical address
    pub fn initiator(&self) -> CecLogicalAddress {
        (self.msg[0] >> 4).try_into().unwrap() // all values have a variant
    }
    /// return the destination's logical address
    pub fn destination(&self) -> CecLogicalAddress {
        (self.msg[0] & 0xf).try_into().unwrap() // all values have a variant
    }
    /// return the opcode of the message, None for poll
    pub fn opcode(&self) -> Option<Result<CecOpcode, TryFromPrimitiveError<CecOpcode>>> {
        if self.len > 1 {
            Some(self.msg[1].try_into())
        } else {
            None
        }
    }
    pub fn parameters(&self) -> &[u8] {
        if self.len > 2 {
            &self.msg[2..self.len as usize]
        } else {
            &[]
        }
    }
    /// return true if this is a broadcast message
    pub fn is_broadcast(&self) -> bool {
        (self.msg[0] & 0xf) == 0xf
    }
    pub fn is_ok(&self) -> bool {
        //(msg->tx_status && !(msg->tx_status & CEC_TX_STATUS_OK))
        if !self.tx_status.is_empty() && !self.tx_status.contains(TxStatus::OK) {
            return false;
        }
        //(msg->rx_status && !(msg->rx_status & CEC_RX_STATUS_OK))
        if !self.rx_status.is_empty() && !self.rx_status.contains(RxStatus::OK) {
            return false;
        }
        //(!msg->tx_status && !msg->rx_status)
        if self.rx_status.is_empty() && self.tx_status.is_empty() {
            return false;
        }
        // !(msg->rx_status & CEC_RX_STATUS_FEATURE_ABORT)
        !self.rx_status.contains(RxStatus::FEATURE_ABORT)
    }
    pub fn init(from: CecLogicalAddress, to: CecLogicalAddress) -> CecMsg {
        let mut m = Self {
            tx_ts: 0,
            rx_ts: 0,
            len: 1,
            timeout: 0,
            sequence: 0,
            flags: 0,
            msg: [0; 16],
            reply: 0,
            rx_status: RxStatus::empty(),
            tx_status: TxStatus::empty(),
            tx_arb_lost_cnt: 0,
            tx_nack_cnt: 0,
            tx_low_drive_cnt: 0,
            tx_error_cnt: 0,
        };
        let f: u8 = from.into();
        let t: u8 = to.into();
        m.msg[0] = f << 4 | t;
        m
    }
}

/*
 * cec_msg_set_reply_to - fill in destination/initiator in a reply message.
 * @msg:        the message structure for the reply
 * @orig:       the original message structure
 *
 * Set the msg destination to the orig initiator and the msg initiator to the
 * orig destination. Note that msg and orig may be the same pointer, in which
 * case the change is done in place.
static inline void cec_msg_set_reply_to(struct cec_msg *msg,
                                        struct cec_msg *orig)
{
        /* The destination becomes the initiator and vice versa */
        msg->msg[0] = (cec_msg_destination(orig) << 4) |
                      cec_msg_initiator(orig);
        msg->reply = msg->timeout = 0;
}
 */
// ---  cec status field  ---
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TxStatus: u8 {
        const OK          = (1 << 0);
        const ARB_LOST    = (1 << 1);
        const NACK        = (1 << 2);
        const LOW_DRIVE   = (1 << 3);
        const ERROR       = (1 << 4);
        const MAX_RETRIES = (1 << 5);
    }
}
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RxStatus: u8 {
        const OK            = (1 << 0);
        const TIMEOUT       = (1 << 1);
        const FEATURE_ABORT = (1 << 2);
    }
}
/*
const CEC_LOG_ADDR_INVALID: u8 = 0xff;
const CEC_PHYS_ADDR_INVALID: u16 = 0xffff;
*/
/**
 * The maximum number of logical addresses one device can be assigned to.
 * The CEC 2.0 spec allows for only 2 logical addresses at the moment. The
 * Analog Devices CEC hardware supports 3. So let's go wild and go for 4.
 */
const CEC_MAX_LOG_ADDRS: usize = 4;

/**
 * The logical addresses defined by CEC 2.0
 * 
 * Switches should use UNREGISTERED.
 * Processors should use SPECIFIC.
 */
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Copy)]
#[repr(u8)]
pub enum CecLogicalAddress {
    Tv = 0,
    Record1 = 1,
    Record2 = 2,
    Tuner1 = 3,
    Playback1 = 4,
    Audiosystem = 5,
    Tuner2 = 6,
    Tuner3 = 7,
    Playback2 = 8,
    Record3 = 9,
    Tuner4 = 10,
    Playback3 = 11,
    Backup1 = 12,
    Backup2 = 13,
    Specific = 14,
    ///as initiator address
    UnregisteredBroadcast = 15,
}

bitflags! {
    /// The bitmask of all logical addresses this adapter has claimed.
    /// 
    /// If this adapter is not configured at all, then log_addr_mask is set to 0.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CecLogAddrMask: u16 {
        const Tv            = (1 << 0);
        const Record1       = (1 << 1);
        const Record2       = (1 << 2);
        const Record3       = (1 << 9);
        const Tuner1        = (1 << 3);
        const Tuner2        = (1 << 6);
        const Tuner3        = (1 << 7);
        const Tuner4        = (1 << 10);
        const Playback1     = (1 << 4);
        const Playback2     = (1 << 8);
        const Playback3     = (1 << 11);
        const Audiosystem   = (1 << 5);
        const Backup1       = (1 << 12);
        const Backup2       = (1 << 13);
        const Specific      = (1 << 14);
        /// adapter is Unregistered
        const Unregistered  = (1 << 15);
    }
}
impl CecLogAddrMask {
    #[inline]
    pub fn is_playback(&self) -> bool {
        self.intersects(Self::Playback1 | Self::Playback2 | Self::Playback3)
    }
    #[inline]
    pub fn is_record(&self) -> bool {
        self.intersects(Self::Record1 | Self::Record2 | Self::Record3)
    }
    #[inline]
    pub fn is_tuner(&self) -> bool {
        self.intersects(Self::Tuner1 | Self::Tuner2 | Self::Tuner3 | Self::Tuner4)
    }
    #[inline]
    pub fn is_backup(&self) -> bool {
        self.intersects(Self::Backup1 | Self::Backup2)
    }
}

// ---  Events  ---
#[repr(u32)]
pub enum CecEventType {
    /// Event that occurs when the adapter state changes
    StateChange = 1,
    /// This event is sent when messages are lost because the application
    /// didn't empty the message queue in time
    LostMsgs = 2,
}
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CecEventFlags : u32 {
        const CEC_EVENT_FL_INITIAL_STATE = (1 << 0);
    }
}

///used when the CEC adapter changes state.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CecEventStateChange {
    ///the current physical address
    pub phys_addr: u16,
    /// The current set of claimed logical addresses.
    /// This is 0 if no logical addresses are claimed or if `phys_addr`` is CEC_PHYS_ADDR_INVALID.
    pub log_addr_mask: CecLogAddrMask,
}

///tells you how many messages were lost due
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CecEventLostMsgs {
    ///how many messages were lost.
    pub lost_msgs: u32,
}
#[repr(C)]
pub union CecEventPayload {
    ///the event payload for CEC_EVENT_STATE_CHANGE.
    pub state_change: CecEventStateChange,
    ///the event payload for CEC_EVENT_LOST_MSGS.
    pub lost_msgs: CecEventLostMsgs,
    ///array to pad the union.
    raw: [u32; 16],
}
#[repr(C)]
pub struct CecEvent {
    ///the timestamp of when the event was sent.
    pub ts: u64,
    pub typ: CecEventType,
    pub flags: CecEventFlags,
    pub payload: CecEventPayload,
}
impl Default for CecEvent {
    fn default() -> Self {
        Self {
            ts: Default::default(),
            typ: CecEventType::LostMsgs,
            flags: CecEventFlags::empty(),
            payload: CecEventPayload { raw: [0; 16] },
        }
    }
}
//#define CEC_DQEVENT             _IOWR('a',  7, struct cec_event)
ioctl_readwrite! {
    /**
     * The internal event queues are per-filehandle and per-event type. If there is no more room in a queue then the last event is overwritten with the new one. This means that intermediate results can be thrown away but that the latest event is always available. This also means that is it possible to read two successive events that have the same value (e.g. two CEC_EVENT_STATE_CHANGE events with the same state). In that case the intermediate state changes were lost but it is guaranteed that the state did change in between the two events.
     */
    get_event, b'a',  7, CecEvent
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Copy)]
#[repr(u8)]
pub enum CecOpcode {
    /* One Touch Play Feature */
    /// Used by a new source to indicate that it has started to transmit a stream OR used in response to a [CecOpcode::RequestActiveSource]  
    /// __Parameters:__ 2byte - physical address of active source
    ActiveSource = 0x82,
    /// Sent by a source device to the TV whenever it enters the active state (alternatively it may send [CecOpcode::TextViewOn]).
    /// The TV should then turn on (if not on). If in ‘Text Display’ state then the TV enters ‘Image Display’ state.
    ImageViewOn = 0x04,
    /// As [CecOpcode::ImageViewOn], but should also remove any text, menus and PIP windows from the TV’s display.
    TextViewOn = 0x0d,
    /* Routing Control Feature */

    /*
     * Has also:
     *      ACTIVE_SOURCE
     */
    /// Used by the currently active source to inform the TV that it has no video to be presented to the user, or is going into standby as the result of a local user command on the device.  
    /// __Parameters:__ 2byte - physical address of active source
    InactiveSource = 0x9d,
    /// Used by a new device to discover the status of the system.
    RequestActiveSource = 0x85,
    /// Sent by a CEC Switch when it is manually switched to inform all other devices on the network that the active route below the switch has changed.  
    /// __Parameters:__
    /// - 2byte - old physical address
    /// - 2byte - new physical address
    RoutingChange = 0x80,
    /// Sent by a CEC Switch to indicate the active route below the switch.  
    /// __Parameters:__ 2byte - physical address
    RoutingInformation = 0x81,
    /// Used by the TV to request a streaming path from the specified physical address.  
    /// __Parameters:__ 2byte - physical address
    SetStreamPath = 0x86,

    /* Standby Feature */
    /// Turn off remote device. Can be used as a broadcast. No Payload
    Standby = 0x36,

    /* System Information Feature */
    /// Used to indicate the supported CEC version, in response to a [CecOpcode::GetCecVersion]  
    /// __Parameters:__ [Version]
    CecVersion = 0x9e,
    /// core message  
    /// When in passthrough mode this message has to be handled by userspace, otherwise the core will return the CEC version that was set with [CecDevice::set_log](super::CecDevice::set_log).
    GetCecVersion = 0x9f,
    /// core message  
    /// When in passthrough mode this message has to be handled by userspace, otherwise the core will report the current physical address.
    GivePhysicalAddr = 0x83,
    GetMenuLanguage = 0x91,
    /// Used to inform all other devices of the mapping between physical and logical address of the initiator.  
    /// __Parameters:__
    /// - 2b physical address
    /// - 1b [Device Type]
    ReportPhysicalAddr = 0x84,
    /// Used by a TV or another device to indicate the menu language.  
    /// __Parameters:__ [Language]
    SetMenuLanguage = 0x32,
    /// HDMI 2.0
    ReportFeatures = 0xa6,

    /* Device Feature Operand (dev_features) */
    /// HDMI 2.0  
    /// core message
    /// When in passthrough mode this message has to be handled by userspace, otherwise the core will report the current features as was set with [CecDevice::set_log](super::CecDevice::set_log) or the message is ignored if the CEC version was older than 2.0.
    GiveFeatures = 0xa5,

    /* Deck Control Feature */
    /// Used to control a device’s media functions.  
    /// __Parameters:__ [DeckControlMode]
    DeckControl = 0x42,
    /// Used to provide a deck’s status to the initiator of the [CecOpcode::GiveDeckStatus] message.  
    /// __Parameters:__ [DeckInfo]
    DeckStatus = 0x1b,
    /// Used to request the status of a device, regardless of whether or not it is the current active source.  
    /// __Parameters:__ [StatusRequest]
    GiveDeckStatus = 0x1a,
    /// Used to control the playback behaviour of a source device.  
    /// __Parameters:__ [PlayMode]
    Play = 0x41,

    /* Vendor Specific Commands Feature */

    /*
     * Has also:
     *      CEC_VERSION
     *      GET_CEC_VERSION
     */
    /// Reports the vendor ID of this device.  
    /// __Parameters:__ [VendorID]
    DeviceVendorId = 0x87,
    /// core message
    /// When in passthrough mode this message has to be handled by userspace, otherwise the core will return the vendor ID that was set with [CecDevice::set_log](super::CecDevice::set_log).
    GiveDeviceVendorId = 0x8c,
    /// Allows vendor specific commands to be sent between two devices.  
    /// __Parameters:__ vendor specific
    VendorCommand = 0x89,
    /// Allows vendor specific commands to be sent between two devices.  
    /// __Parameters:__
    /// - [VendorID]
    /// - vendor specific
    VendorCommandWithId = 0xa0,
    /// Indicates that a remote control button has been depressed.
    /// __Parameters:__ Vendor Specific RC Code
    VendorRemoteButtonDown = 0x8a,
    /// The last button pressed indicated by the [CecOpcode::VendorRemoteButtonDown] message has been released.
    VendorRemoteButtonUp = 0x8b,

    /* OSD Display Feature */
    /// Used to send a text message to output on a TV.  
    /// __Parameters:__
    /// - [DisplayControl]
    /// - [OSDStr<13>] String not terminated or prefixed by anything
    SetOsdString = 0x64,
    /* Device OSD Transfer Feature */
    /// core message
    /// When in passthrough mode this message has to be handled by userspace, otherwise the core will report the current OSD name as was set with [CecDevice::set_log](super::CecDevice::set_log).
    /// No payload. Requests a [CecOpcode::SetOsdName]
    GiveOsdName = 0x46,
    /// answer to [CecOpcode::GiveOsdName].  
    /// __Parameters:__
    /// [OSDStr<14>] the name of the device (used in menus). not terminated or prefixed by anything
    SetOsdName = 0x47,

    /* Device Menu Control Feature */
    /// A request from the TV for a device to show/remove a menu or to query if a device is currently showing a menu.  
    /// __Parameters:__ [MenuRequestType]
    MenuRequest = 0x8d,
    /// Used to indicate to the TV that the device is showing/has removed a menu and requests the remote control keys to be passed though.  
    /// __Parameters:__ 1 byte Activated(0)/Deactivated(1)
    MenuStatus = 0x8e,
    /* Menu State Operand (menu_state) */
    /// Used to indicate that the user pressed a remote control button or switched from one remote control button to another.  
    /// __Parameters:__ 1 byte [CecUserControlCode]
    UserControlPressed = 0x44,
    /// The last button pressed indicated by the [CecOpcode::UserControlPressed] message has been released.
    UserControlReleased = 0x45,

    /* Remote Control Passthrough Feature */

    /*
     * Has also:
     *      USER_CONTROL_PRESSED
     *      USER_CONTROL_RELEASED
     */

    /* Power Status Feature */
    /// request [CecOpcode::ReportPowerStatus]
    GiveDevicePowerStatus = 0x8f,
    /// Answer to [CecOpcode::GiveDevicePowerStatus]
    ///
    /// __Parameters:__ 1 byte [CecPowerStatus]
    ReportPowerStatus = 0x90,
    /* General Protocol Messages */
    /**
     * It is used to allow devices to indicate if they do not
     * support an opcode that has been directly sent to them, if it is unable to deal with the message at present, or if there
     * was something wrong with the transmitted frame at the high-level protocol layer.
     *
     * __Parameters:__
     * - [CecOpcode]
     * - [CecAbortReason]
     */
    FeatureAbort = 0x00,
    /// When in [CecModeFollower::ExclusivePassthru] this message has to be handled by userspace, otherwise the core will return a feature refused message as per the specification.
    Abort = 0xff,

    /* System Audio Control Feature */

    /*
     * Has also:
     *      USER_CONTROL_PRESSED
     *      USER_CONTROL_RELEASED
     */
    /// Requests an amplifier to send its volume and mute status via [CecOpcode::ReportAudioStatus]
    GiveAudioStatus = 0x71,
    /// Requests the status of the [System Audio Mode](CecOpcode::SystemAudioModeStatus)
    GiveSystemAudioModeStatus = 0x7d,
    /**
     * Used to indicate the current audio volume status of a device.  
     * __Parameters:__ 1 byte
     *
     * Payload indicates audio playback volume, expressed as a percentage
     * (0% - 100%). N=0 is no sound; N=100 is maximum volume
     * sound level.
     * The linearity of the sound level is device dependent.
     * This value is mainly used for displaying a volume status bar on
     * a TV screen.
     *
     * The payload's highest bit (`&0x80`) indicates mute
     */
    ReportAudioStatus = 0x7a,
    ReportShortAudioDescriptor = 0xa3,
    RequestShortAudioDescriptor = 0xa4,
    /// Turns the System Audio Mode On or Off.  
    /// __Parameters:__ 1 byte On(1)/Off(0)
    /// If set to On, the TV mutes its speakers. The TV or STB sends relevant [CecOpcode::UserControlPressed] or [CecOpcode::UserControlReleased] as necessary.
    SetSystemAudioMode = 0x72,

    /**
     * Requests to use [System Audio Mode](CecOpcode::SystemAudioModeStatus) to the amplifier
     *
     * __Parameters:__
     * 2b physical address of device to be used as source of the audio stream.
     * **OR**:  
     * no payload
     *
     *
     * The amplifier comes out of standby (if necessary) and switches to the relevant connector for device specified by Physical Address.
     * It then sends a [CecOpcode::SetSystemAudioMode] `On` message.
     *
     * ...  the device requesting this information can send the volume-related [CecOpcode::UserControlPressed] or [CecOpcode::UserControlReleased] messages.
     *
     * [CecOpcode::SystemAudioModeRequest] sent without a Physical Address parameter requests termination of the feature.
     * In this case, the amplifier sends a [CecOpcode::SetSystemAudioMode] `Off` message.
     */
    SystemAudioModeRequest = 0x70,

    /**
     *  Reports the current status of the System Audio Mode
     *
     * __Parameters:__ 1 byte On(1)/Off(0)
     *
     * The feature can be initiated from a device (eg TV or STB) or the amplifier. In the case of initiation by a device
     * other than the amplifier, that device sends an [CecOpcode::SystemAudioModeRequest] to the amplifier, with the
     * physical address of the device that it wants to use as a source as an operand. Note that the Physical Address
     * may be the TV or STB itself.
     *
     */
    SystemAudioModeStatus = 0x7e,
    /* Audio Rate Control Feature */
    /// Used to control audio rate from Source Device.  
    /// __Parameters:__ [Audio Rate]
    SetAudioRate = 0x9a,

    /* One Touch Record Feature */
    /// Requests a device to stop a recording.
    RecordOff = 0x0b,
    /// Attempt to record the specified source.  
    /// __Parameters:__ [RecordSource]
    RecordOn = 0x09,
    /// Used by a Recording Device to inform the initiator of the message [CecOpcode::RecordOn] about its status.  
    /// __Parameters:__ [RecordStatusInfo]
    RecordStatus = 0x0a,
    /// Request by the Recording Device to record the presently displayed source.
    RecordTvScreen = 0x0f,

    /* Timer Programming Feature */
    /// Used to clear an Analogue timer block of a device.  
    /// __Parameters:__ See [CecOpcode::SetAnalogueTimer]
    ClearAnalogueTimer = 0x33,
    /// Used to clear an Digital timer block of a device.  
    /// __Parameters:__ See [CecOpcode::SetDigitalTimer]
    ClearDigitalTimer = 0x99,
    /// Used to clear an External timer block of a device.  
    /// __Parameters:__ See [CecOpcode::SetExtTimer]
    ClearExtTimer = 0xa1,
    /// Used to set a single timer block on an Analogue Recording Device.  
    /// __Parameters:__
    /// - [CecTimer]
    /// - [RecordingSequence]
    /// - [Analogue Broadcast Type]
    /// - [Analogue Frequency]
    /// - [Broadcast System]
    SetAnalogueTimer = 0x34,
    /// Used to set a single timer block on a Digital Recording Device.  
    /// __Parameters:__
    /// - [CecTimer]
    /// - [RecordingSequence]
    /// - [Digital Service Identification]
    SetDigitalTimer = 0x97,
    /// Used to set a single timer block to record from an external device.  
    /// __Parameters:__
    /// - [CecTimer]
    /// - [RecordingSequence]
    /// - [External Source Specifier]
    /// - [External Plug] | [External Physical Address]
    SetExtTimer = 0xa2,
    /// Used to set the name of a program associated with a timer block.
    /// Sent directly after sending a [CecOpcode::SetAnalogueTimer] or [CecOpcode::SetDigitalTimer] message.
    /// The name is then associated with that timer block.  
    /// __Parameters:__ [Program Title String]
    SetTimerProgramTitle = 0x67,
    /// Used to give the status of a [CecOpcode::ClearAnalogueTimer], [CecOpcode::ClearDigitalTimer] or [CecOpcode::ClearExtTimer] message.  
    /// __Parameters:__ [TimerClearedStatusData]
    TimerClearedStatus = 0x43,
    /// Used to send timer status to the initiator of a [CecOpcode::SetAnalogueTimer], [CecOpcode::SetDigitalTimer] or [CecOpcode::SetExtTimer] message.  
    /// __Parameters:__ [TimerStatusData]
    TimerStatus = 0x35,

    /* Tuner Control Feature */
    /// Used to request the status of a tuner device.  
    /// __Parameters:__ [StatusRequest]
    GiveTunerDeviceStatus = 0x08,
    SelectAnalogueService = 0x92,
    SelectDigitalService = 0x93,
    TunerDeviceStatus = 0x07,
    TunerStepDecrement = 0x06,
    TunerStepIncrement = 0x05,

    /* Audio Return Channel Control Feature */
    InitiateArc = 0xc0,
    ReportArcInitiated = 0xc1,
    ReportArcTerminated = 0xc2,
    RequestArcInitiation = 0xc3,
    RequestArcTermination = 0xc4,
    TerminateArc = 0xc5,

    /* Dynamic Audio Lipsync Feature */
    /* Only for CEC 2.0 and up */
    RequestCurrentLatency = 0xa7,
    ReportCurrentLatency = 0xa8,
    /* Capability Discovery and Control Feature */
    CdcMessage = 0xf8,
}
/// parameter for [CecOpcode::UserControlPressed]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Copy)]
#[repr(u8)]
pub enum CecUserControlCode {
    Select = 0x00,
    Up = 0x01,
    Down = 0x02,
    Left = 0x03,
    Right = 0x04,
    RightUp = 0x05,
    RightDown = 0x06,
    LeftUp = 0x07,
    LeftDown = 0x08,
    RootMenu = 0x09,
    SetupMenu = 0x0a,
    ContentsMenu = 0x0b,
    FavoriteMenu = 0x0c,
    Exit = 0x0d,
    // reserved: 0x0E, 0x0F
    TopMenu = 0x10,
    DvdMenu = 0x11, // reserved: 0x12 ... 0x1C
    NumberEntryMode = 0x1d,
    Number11 = 0x1e,
    Number12 = 0x1f,
    Number0 = 0x20,
    Number1 = 0x21,
    Number2 = 0x22,
    Number3 = 0x23,
    Number4 = 0x24,
    Number5 = 0x25,
    Number6 = 0x26,
    Number7 = 0x27,
    Number8 = 0x28,
    Number9 = 0x29,
    Dot = 0x2a,
    Enter = 0x2b,
    Clear = 0x2c,
    NextFavorite = 0x2f,
    ChannelUp = 0x30,
    ChannelDown = 0x31,
    PreviousChannel = 0x32,
    SoundSelect = 0x33,
    InputSelect = 0x34,
    DisplayInformation = 0x35,
    Help = 0x36,
    PageUp = 0x37,
    PageDown = 0x38,
    // reserved: 0x39 ... 0x3F
    Power = 0x40,
    VolumeUp = 0x41,
    VolumeDown = 0x42,
    Mute = 0x43,
    Play = 0x44,
    Stop = 0x45,
    Pause = 0x46,
    Record = 0x47,
    Rewind = 0x48,
    FastForward = 0x49,
    Eject = 0x4a,
    Forward = 0x4b,
    Backward = 0x4c,
    StopRecord = 0x4d,
    PauseRecord = 0x4e,
    // reserved: 0x4F
    Angle = 0x50,
    SubPicture = 0x51,
    VideoOnDemand = 0x52,
    ElectronicProgramGuide = 0x53,
    TimerProgramming = 0x54,
    InitialConfiguration = 0x55,
    SelectBroadcastType = 0x56,
    SelectSoundPresentation = 0x57,
    // reserved: 0x58 ... 0x5F
    /// Additional Operands: [Play Mode]
    PlayFunction = 0x60,
    PausePlayFunction = 0x61,
    RecordFunction = 0x62,
    PauseRecordFunction = 0x63,
    StopFunction = 0x64,
    MuteFunction = 0x65,
    RestoreVolumeFunction = 0x66,
    /// Additional Operands: [Channel Identifier]
    TuneFunction = 0x67,
    /// Additional Operands: [UI Function Media]
    SelectMediaFunction = 0x68,
    /// Additional Operands: [UI Function Select A/V input]
    SelectAvInputFunction = 0x69,
    /// Additional Operands: [UI Function Select Audio input]
    SelectAudioInputFunction = 0x6a,
    PowerToggleFunction = 0x6b,
    PowerOffFunction = 0x6c,
    PowerOnFunction = 0x6d,
    // reserved: 0x6E ... 0x70
    F1Blue = 0x71,
    F2Red = 0x72,
    F3Green = 0x73,
    F4Yellow = 0x74,
    F5 = 0x75,
    Data = 0x76,
    // reserved: 0x77 ... 0xFF
}
/// used by [CecOpcode::FeatureAbort]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Copy)]
#[repr(u8)]
pub enum CecAbortReason {
    /// Unrecognized opcode
    Unrecognized = 0,
    /// Not in correct mode to respond
    WrongMode = 1,
    /// Cannot provide source
    NoSource = 2,
    /// Invalid operand
    InvalidOp = 3,
    Refused = 4,
    Other = 5,
}
/// used by [CecOpcode::DeckControl]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Copy)]
#[repr(u8)]
pub enum DeckControlMode {
    Skip = 1,
    Rewind = 2,
    Stop = 3,
    Eject = 4,
}
/// used by [CecOpcode::DeckStatus]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Copy)]
#[repr(u8)]
pub enum DeckInfo {
    Play = 0x11,
    Record = 0x12,
    PlayRev = 0x13,
    Still = 0x14,
    Slow = 0x15,
    SlowRev = 0x16,
    FastFwd = 0x17,
    FastRev = 0x18,
    NoMedia = 0x19,
    Stop = 0x1a,
    SkipFwd = 0x1b,
    SkipRev = 0x1c,
    IndexSearchFwd = 0x1d,
    IndexSearchRev = 0x1e,
    Other = 0x1f,
}
/// used by [CecOpcode::SetOsdString]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Copy)]
#[repr(u8)]
pub enum DisplayControl {
    Default = 0x00,
    UntilCleared = 0x40,
    Clear = 0x80,
}
/// used by [CecOpcode::MenuRequest]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Copy)]
#[repr(u8)]
pub enum MenuRequestType {
    Activate = 0x00,
    Deactivate = 0x01,
    Query = 0x02,
}
/// used by [CecOpcode::Play]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Copy)]
#[repr(u8)]
pub enum PlayMode {
    Fwd = 0x24,
    Rev = 0x20,
    Still = 0x25,
    FastFwdMin = 0x05,
    FastFwdMed = 0x06,
    FastFwdMax = 0x07,
    FastRevMin = 0x09,
    FastRevMed = 0x0a,
    FastRevMax = 0x0b,
    SlowFwdMin = 0x15,
    SlowFwdMed = 0x16,
    SlowFwdMax = 0x17,
    SlowRevMin = 0x19,
    SlowRevMed = 0x1a,
    SlowRevMax = 0x1b,
}
/// used by [CecOpcode::GiveDeckStatus] and [CecOpcode::GiveTunerDeviceStatus]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Copy)]
#[repr(u8)]
pub enum StatusRequest {
    On = 1,
    Off = 2,
    Once = 3,
}
/*
#[repr(transparent)]
pub struct Volume(u8);
impl Volume {
    pub fn vol(&self) -> Option<u8> {
        let v = self.0 & 0x7f;
        //0x65 ..= 0x7E Reserved
        if v == 0x7f {
            return None;
        }
        Some(v)
    }
    pub fn is_mute(&self) -> bool {
        self.0 & 0x80 == 0x80
    }
}
*/

/// Payload of [CecOpcode::SetAnalogueTimer], [CecOpcode::SetDigitalTimer] or [CecOpcode::SetExtTimer]
#[repr(C)]
pub struct CecTimer {
    /// Day of Month: 1 byte 1..=31
    pub day: u8,
    /// Month of Year: 1 byte 1..=12
    pub month: u8,
    /// Start Hour: 1 byte 0..=23
    pub start_h: u8,
    /// Start Minute: 1 byte 0..=59
    pub start_min: u8,
    /// Duration Hours: 1 byte 1..=99
    pub duration_h: u8,
    /// Duration Minutes: 1 byte 0..=59
    pub duration_min: u8,
}

#[repr(transparent)]
pub struct VendorID(pub [u8; 3]);
/*
 * Use this if there is no vendor ID (CEC_G_VENDOR_ID) or if the vendor ID
 * should be disabled (CEC_S_VENDOR_ID)
 */
pub const CEC_VENDOR_ID_NONE: u32 = 0xffffffff;

bitflags! {
    /// Repeat recording or don't (if zero)
    ///
    /// Payload of [CecOpcode::SetAnalogueTimer], [CecOpcode::SetDigitalTimer] or [CecOpcode::SetExtTimer]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RecordingSequence : u8 {
        const SUNDAY = 0x01;
        const MONDAY = 0x02;
        const TUESDAY = 0x04;
        const WEDNESDAY = 0x08;
        const THURSDAY = 0x10;
        const FRIDAY = 0x20;
        const SATURDAY = 0x40;
    }
}

// ---  Power Status Operand (pwr_state)  ---
/// Payload of [CecOpcode::ReportPowerStatus]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Copy)]
#[repr(u8)]
pub enum CecPowerStatus {
    On = 0,
    Standby = 1,
    InTransitionStandbyToOn = 2,
    InTransitionOnToStandby = 3,
}

//use std::ffi::c_char;
#[allow(non_camel_case_types)]
type c_char = u8; //its actually i8, but that sucks

/**
 * Create it from a String (String has to be ascii)
 * ```
 * # use cec_linux::OSDStr;
 * let name: OSDStr::<15> = "pi4".to_string().try_into().unwrap();
 * ```
 *
 * and use it as `&str`
 * ```
 * # use cec_linux::OSDStr;
 * let str: &str = OSDStr::<14>::default().as_ref();
 * ```
 */
#[repr(transparent)]
#[derive(Clone)]
pub struct OSDStr<const MAX: usize>([c_char; MAX]);

// from CecMsg to OSDStr
impl<const MAX: usize> From<&[u8]> for OSDStr<MAX> {
    fn from(value: &[u8]) -> Self {
        let mut osd = OSDStr::default();
        let len = MAX.min(value.len());
        osd.0[..len].clone_from_slice(value);
        osd
    }
}

// from String to OSDStr
impl<const MAX: usize> TryFrom<String> for OSDStr<MAX> {
    type Error = ();
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            let mut v = value.into_bytes();
            v.resize(MAX, 0);
            let a = v.try_into().unwrap(); //len is ok
            return Ok(OSDStr(a));
        }
        Err(())
    }
}

// from OSDStr to &str
impl<const MAX: usize> AsRef<str> for OSDStr<MAX> {
    fn as_ref(&self) -> &str {
        match std::ffi::CStr::from_bytes_until_nul(&self.0) {
            Ok(s) => s.to_str().unwrap_or_default(),
            Err(_) => {
                //no terminating null
                std::str::from_utf8(&self.0).unwrap_or_default()
            }
        }
    }
}
/*impl<const MAX: usize> std::ops::Deref for OSDStr<MAX> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        match std::ffi::CStr::from_bytes_until_nul(&self.0) {
            Ok(s) => s.to_str().unwrap_or_default(),
            Err(_) => {
                //no terminating null
                std::str::from_utf8(&self.0).unwrap_or_default()
            }
        }
    }
}*/
impl<const MAX: usize> std::fmt::Display for OSDStr<MAX> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
impl<const MAX: usize> std::fmt::Debug for OSDStr<MAX> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl<const MAX: usize> Default for OSDStr<MAX> {
    fn default() -> Self {
        Self([0; MAX])
    }
}

/*
// --- Ethernet-over-HDMI: nobody ever does this... ---
const CEC_MSG_CDC_HEC_INQUIRE_STATE: u8 = 0x00;
const CEC_MSG_CDC_HEC_REPORT_STATE: u8 = 0x01;
const CEC_MSG_CDC_HEC_SET_STATE_ADJACENT: u8 = 0x02;
const CEC_MSG_CDC_HEC_SET_STATE: u8 = 0x03;

const CEC_MSG_CDC_HEC_REQUEST_DEACTIVATION: u8 = 0x04;
const CEC_MSG_CDC_HEC_NOTIFY_ALIVE: u8 = 0x05;
const CEC_MSG_CDC_HEC_DISCOVER: u8 = 0x06;
// --- Hotplug Detect messages ---
const CEC_MSG_CDC_HPD_SET_STATE: u8 = 0x10;
// ---  HPD State Operand (hpd_state)  ---
const CEC_MSG_CDC_HPD_REPORT_STATE: u8 = 0x11;

// ---  Record Source Type Operand (rec_src_type)  ---
const CEC_OP_RECORD_SRC_OWN: u8 = 1;
const CEC_OP_RECORD_SRC_DIGITAL: u8 = 2;
const CEC_OP_RECORD_SRC_ANALOG: u8 = 3;
const CEC_OP_RECORD_SRC_EXT_PLUG: u8 = 4;
const CEC_OP_RECORD_SRC_EXT_PHYS_ADDR: u8 = 5;
// ---  Service Identification Method Operand (service_id_method)  ---
const CEC_OP_SERVICE_ID_METHOD_BY_DIG_ID: u8 = 0;
const CEC_OP_SERVICE_ID_METHOD_BY_CHANNEL: u8 = 1;
// ---  Digital Service Broadcast System Operand (dig_bcast_system)  ---
const CEC_OP_DIG_SERVICE_BCAST_SYSTEM_ARIB_GEN: u8 = 0x00;
const CEC_OP_DIG_SERVICE_BCAST_SYSTEM_ATSC_GEN: u8 = 0x01;
const CEC_OP_DIG_SERVICE_BCAST_SYSTEM_DVB_GEN: u8 = 0x02;
const CEC_OP_DIG_SERVICE_BCAST_SYSTEM_ARIB_BS: u8 = 0x08;
const CEC_OP_DIG_SERVICE_BCAST_SYSTEM_ARIB_CS: u8 = 0x09;
const CEC_OP_DIG_SERVICE_BCAST_SYSTEM_ARIB_T: u8 = 0x0a;
const CEC_OP_DIG_SERVICE_BCAST_SYSTEM_ATSC_CABLE: u8 = 0x10;
const CEC_OP_DIG_SERVICE_BCAST_SYSTEM_ATSC_SAT: u8 = 0x11;
const CEC_OP_DIG_SERVICE_BCAST_SYSTEM_ATSC_T: u8 = 0x12;
const CEC_OP_DIG_SERVICE_BCAST_SYSTEM_DVB_C: u8 = 0x18;
const CEC_OP_DIG_SERVICE_BCAST_SYSTEM_DVB_S: u8 = 0x19;
const CEC_OP_DIG_SERVICE_BCAST_SYSTEM_DVB_S2: u8 = 0x1a;
const CEC_OP_DIG_SERVICE_BCAST_SYSTEM_DVB_T: u8 = 0x1b;
// ---  Analogue Broadcast Type Operand (ana_bcast_type)  ---
const CEC_OP_ANA_BCAST_TYPE_CABLE: u8 = 0;
const CEC_OP_ANA_BCAST_TYPE_SATELLITE: u8 = 1;
const CEC_OP_ANA_BCAST_TYPE_TERRESTRIAL: u8 = 2;
// ---  Broadcast System Operand (bcast_system)  ---
const CEC_OP_BCAST_SYSTEM_PAL_BG: u8 = 0x00;
const CEC_OP_BCAST_SYSTEM_SECAM_LQ: u8 = 0x01; // * SECAM L' *
const CEC_OP_BCAST_SYSTEM_PAL_M: u8 = 0x02;
const CEC_OP_BCAST_SYSTEM_NTSC_M: u8 = 0x03;
const CEC_OP_BCAST_SYSTEM_PAL_I: u8 = 0x04;
const CEC_OP_BCAST_SYSTEM_SECAM_DK: u8 = 0x05;
const CEC_OP_BCAST_SYSTEM_SECAM_BG: u8 = 0x06;
const CEC_OP_BCAST_SYSTEM_SECAM_L: u8 = 0x07;
const CEC_OP_BCAST_SYSTEM_PAL_DK: u8 = 0x08;
const CEC_OP_BCAST_SYSTEM_OTHER: u8 = 0x1f;
// ---  Channel Number Format Operand (channel_number_fmt)  ---
const CEC_OP_CHANNEL_NUMBER_FMT_1_PART: u8 = 0x01;
const CEC_OP_CHANNEL_NUMBER_FMT_2_PART: u8 = 0x02;

// ---  Record Status Operand (rec_status)  ---
const CEC_OP_RECORD_STATUS_CUR_SRC: u8 = 0x01;
const CEC_OP_RECORD_STATUS_DIG_SERVICE: u8 = 0x02;
const CEC_OP_RECORD_STATUS_ANA_SERVICE: u8 = 0x03;
const CEC_OP_RECORD_STATUS_EXT_INPUT: u8 = 0x04;
const CEC_OP_RECORD_STATUS_NO_DIG_SERVICE: u8 = 0x05;
const CEC_OP_RECORD_STATUS_NO_ANA_SERVICE: u8 = 0x06;
const CEC_OP_RECORD_STATUS_NO_SERVICE: u8 = 0x07;
const CEC_OP_RECORD_STATUS_INVALID_EXT_PLUG: u8 = 0x09;
const CEC_OP_RECORD_STATUS_INVALID_EXT_PHYS_ADDR: u8 = 0x0a;
const CEC_OP_RECORD_STATUS_UNSUP_CA: u8 = 0x0b;
const CEC_OP_RECORD_STATUS_NO_CA_ENTITLEMENTS: u8 = 0x0c;
const CEC_OP_RECORD_STATUS_CANT_COPY_SRC: u8 = 0x0d;
const CEC_OP_RECORD_STATUS_NO_MORE_COPIES: u8 = 0x0e;
const CEC_OP_RECORD_STATUS_NO_MEDIA: u8 = 0x10;
const CEC_OP_RECORD_STATUS_PLAYING: u8 = 0x11;
const CEC_OP_RECORD_STATUS_ALREADY_RECORDING: u8 = 0x12;
const CEC_OP_RECORD_STATUS_MEDIA_PROT: u8 = 0x13;
const CEC_OP_RECORD_STATUS_NO_SIGNAL: u8 = 0x14;
const CEC_OP_RECORD_STATUS_MEDIA_PROBLEM: u8 = 0x15;
const CEC_OP_RECORD_STATUS_NO_SPACE: u8 = 0x16;
const CEC_OP_RECORD_STATUS_PARENTAL_LOCK: u8 = 0x17;
const CEC_OP_RECORD_STATUS_TERMINATED_OK: u8 = 0x1a;
const CEC_OP_RECORD_STATUS_ALREADY_TERM: u8 = 0x1b;
const CEC_OP_RECORD_STATUS_OTHER: u8 = 0x1f;


// ---  External Source Specifier Operand (ext_src_spec)  ---
const CEC_OP_EXT_SRC_PLUG: u8 = 0x04;
const CEC_OP_EXT_SRC_PHYS_ADDR: u8 = 0x05;

// ---  Timer Cleared Status Data Operand (timer_cleared_status)  ---
const CEC_OP_TIMER_CLR_STAT_RECORDING: u8 = 0x00;
const CEC_OP_TIMER_CLR_STAT_NO_MATCHING: u8 = 0x01;
const CEC_OP_TIMER_CLR_STAT_NO_INFO: u8 = 0x02;
const CEC_OP_TIMER_CLR_STAT_CLEARED: u8 = 0x80;

// ---  Timer Overlap Warning Operand (timer_overlap_warning)  ---
const CEC_OP_TIMER_OVERLAP_WARNING_NO_OVERLAP: u8 = 0;
const CEC_OP_TIMER_OVERLAP_WARNING_OVERLAP: u8 = 1;
// ---  Media Info Operand (media_info)  ---
const CEC_OP_MEDIA_INFO_UNPROT_MEDIA: u8 = 0;
const CEC_OP_MEDIA_INFO_PROT_MEDIA: u8 = 1;
const CEC_OP_MEDIA_INFO_NO_MEDIA: u8 = 2;
// ---  Programmed Indicator Operand (prog_indicator)  ---
const CEC_OP_PROG_IND_NOT_PROGRAMMED: u8 = 0;
const CEC_OP_PROG_IND_PROGRAMMED: u8 = 1;
// ---  Programmed Info Operand (prog_info)  ---
const CEC_OP_PROG_INFO_ENOUGH_SPACE: u8 = 0x08;
const CEC_OP_PROG_INFO_NOT_ENOUGH_SPACE: u8 = 0x09;
const CEC_OP_PROG_INFO_MIGHT_NOT_BE_ENOUGH_SPACE: u8 = 0x0b;
const CEC_OP_PROG_INFO_NONE_AVAILABLE: u8 = 0x0a;
// ---  Not Programmed Error Info Operand (prog_error)  ---
const CEC_OP_PROG_ERROR_NO_FREE_TIMER: u8 = 0x01;
const CEC_OP_PROG_ERROR_DATE_OUT_OF_RANGE: u8 = 0x02;
const CEC_OP_PROG_ERROR_REC_SEQ_ERROR: u8 = 0x03;
const CEC_OP_PROG_ERROR_INV_EXT_PLUG: u8 = 0x04;
const CEC_OP_PROG_ERROR_INV_EXT_PHYS_ADDR: u8 = 0x05;
const CEC_OP_PROG_ERROR_CA_UNSUPP: u8 = 0x06;
const CEC_OP_PROG_ERROR_INSUF_CA_ENTITLEMENTS: u8 = 0x07;
const CEC_OP_PROG_ERROR_RESOLUTION_UNSUPP: u8 = 0x08;
const CEC_OP_PROG_ERROR_PARENTAL_LOCK: u8 = 0x09;
const CEC_OP_PROG_ERROR_CLOCK_FAILURE: u8 = 0x0a;
const CEC_OP_PROG_ERROR_DUPLICATE: u8 = 0x0e;

// ---  Valid for RC Profile and Device Feature operands  ---
const CEC_OP_FEAT_EXT: u8 = 0x80; //   / * Extension bit *
                                  / * RC Profile Operand (rc_profile) * /
const CEC_OP_FEAT_RC_TV_PROFILE_NONE: u8 = 0x00;
const CEC_OP_FEAT_RC_TV_PROFILE_1: u8 = 0x02;
const CEC_OP_FEAT_RC_TV_PROFILE_2: u8 = 0x06;
const CEC_OP_FEAT_RC_TV_PROFILE_3: u8 = 0x0a;
const CEC_OP_FEAT_RC_TV_PROFILE_4: u8 = 0x0e;
const CEC_OP_FEAT_RC_SRC_HAS_DEV_ROOT_MENU: u8 = 0x50;
const CEC_OP_FEAT_RC_SRC_HAS_DEV_SETUP_MENU: u8 = 0x48;
const CEC_OP_FEAT_RC_SRC_HAS_CONTENTS_MENU: u8 = 0x44;
const CEC_OP_FEAT_RC_SRC_HAS_MEDIA_TOP_MENU: u8 = 0x42;
const CEC_OP_FEAT_RC_SRC_HAS_MEDIA_CONTEXT_MENU: u8 = 0x41;
// ---  Device Feature Operand (dev_features)  ---
const CEC_OP_FEAT_DEV_HAS_RECORD_TV_SCREEN: u8 = 0x40;
const CEC_OP_FEAT_DEV_HAS_SET_OSD_STRING: u8 = 0x20;
const CEC_OP_FEAT_DEV_HAS_DECK_CONTROL: u8 = 0x10;
const CEC_OP_FEAT_DEV_HAS_SET_AUDIO_RATE: u8 = 0x08;
const CEC_OP_FEAT_DEV_SINK_HAS_ARC_TX: u8 = 0x04;
const CEC_OP_FEAT_DEV_SOURCE_HAS_ARC_RX: u8 = 0x02;


// ---  Recording Flag Operand (rec_flag)  ---
const CEC_OP_REC_FLAG_USED: u8 = 0;
const CEC_OP_REC_FLAG_NOT_USED: u8 = 1;
// ---  Tuner Display Info Operand (tuner_display_info)  ---
const CEC_OP_TUNER_DISPLAY_INFO_DIGITAL: u8 = 0;
const CEC_OP_TUNER_DISPLAY_INFO_NONE: u8 = 1;
const CEC_OP_TUNER_DISPLAY_INFO_ANALOGUE: u8 = 2;


// ---  UI Broadcast Type Operand (ui_bcast_type)  ---
const CEC_OP_UI_BCAST_TYPE_TOGGLE_ALL: u8 = 0x00;
const CEC_OP_UI_BCAST_TYPE_TOGGLE_DIG_ANA: u8 = 0x01;
const CEC_OP_UI_BCAST_TYPE_ANALOGUE: u8 = 0x10;
const CEC_OP_UI_BCAST_TYPE_ANALOGUE_T: u8 = 0x20;
const CEC_OP_UI_BCAST_TYPE_ANALOGUE_CABLE: u8 = 0x30;
const CEC_OP_UI_BCAST_TYPE_ANALOGUE_SAT: u8 = 0x40;
const CEC_OP_UI_BCAST_TYPE_DIGITAL: u8 = 0x50;
const CEC_OP_UI_BCAST_TYPE_DIGITAL_T: u8 = 0x60;
const CEC_OP_UI_BCAST_TYPE_DIGITAL_CABLE: u8 = 0x70;
const CEC_OP_UI_BCAST_TYPE_DIGITAL_SAT: u8 = 0x80;
const CEC_OP_UI_BCAST_TYPE_DIGITAL_COM_SAT: u8 = 0x90;
const CEC_OP_UI_BCAST_TYPE_DIGITAL_COM_SAT2: u8 = 0x91;
const CEC_OP_UI_BCAST_TYPE_IP: u8 = 0xa0;
// ---  UI Sound Presentation Control Operand (ui_snd_pres_ctl)  ---
const CEC_OP_UI_SND_PRES_CTL_DUAL_MONO: u8 = 0x10;
const CEC_OP_UI_SND_PRES_CTL_KARAOKE: u8 = 0x20;
const CEC_OP_UI_SND_PRES_CTL_DOWNMIX: u8 = 0x80;
const CEC_OP_UI_SND_PRES_CTL_REVERB: u8 = 0x90;
const CEC_OP_UI_SND_PRES_CTL_EQUALIZER: u8 = 0xa0;
const CEC_OP_UI_SND_PRES_CTL_BASS_UP: u8 = 0xb1;
const CEC_OP_UI_SND_PRES_CTL_BASS_NEUTRAL: u8 = 0xb2;
const CEC_OP_UI_SND_PRES_CTL_BASS_DOWN: u8 = 0xb3;
const CEC_OP_UI_SND_PRES_CTL_TREBLE_UP: u8 = 0xc1;
const CEC_OP_UI_SND_PRES_CTL_TREBLE_NEUTRAL: u8 = 0xc2;
const CEC_OP_UI_SND_PRES_CTL_TREBLE_DOWN: u8 = 0xc3;

// ---  Audio Format ID Operand (audio_format_id)  ---
const CEC_OP_AUD_FMT_ID_CEA861: u8 = 0;
const CEC_OP_AUD_FMT_ID_CEA861_CXT: u8 = 1;

// ---  Audio Rate Operand (audio_rate)  ---
const CEC_OP_AUD_RATE_OFF: u8 = 0;
const CEC_OP_AUD_RATE_WIDE_STD: u8 = 1;
const CEC_OP_AUD_RATE_WIDE_FAST: u8 = 2;
const CEC_OP_AUD_RATE_WIDE_SLOW: u8 = 3;
const CEC_OP_AUD_RATE_NARROW_STD: u8 = 4;
const CEC_OP_AUD_RATE_NARROW_FAST: u8 = 5;
const CEC_OP_AUD_RATE_NARROW_SLOW: u8 = 6;

// ---  Low Latency Mode Operand (low_latency_mode)  ---
const CEC_OP_LOW_LATENCY_MODE_OFF: u8 = 0;
const CEC_OP_LOW_LATENCY_MODE_ON: u8 = 1;
// ---  Audio Output Compensated Operand (audio_out_compensated)  ---
const CEC_OP_AUD_OUT_COMPENSATED_NA: u8 = 0;
const CEC_OP_AUD_OUT_COMPENSATED_DELAY: u8 = 1;
const CEC_OP_AUD_OUT_COMPENSATED_NO_DELAY: u8 = 2;
const CEC_OP_AUD_OUT_COMPENSATED_PARTIAL_DELAY: u8 = 3;

// ---  HEC Functionality State Operand (hec_func_state)  ---
const CEC_OP_HEC_FUNC_STATE_NOT_SUPPORTED: u8 = 0;
const CEC_OP_HEC_FUNC_STATE_INACTIVE: u8 = 1;
const CEC_OP_HEC_FUNC_STATE_ACTIVE: u8 = 2;
const CEC_OP_HEC_FUNC_STATE_ACTIVATION_FIELD: u8 = 3;
// ---  Host Functionality State Operand (host_func_state)  ---
const CEC_OP_HOST_FUNC_STATE_NOT_SUPPORTED: u8 = 0;
const CEC_OP_HOST_FUNC_STATE_INACTIVE: u8 = 1;
const CEC_OP_HOST_FUNC_STATE_ACTIVE: u8 = 2;
// ---  ENC Functionality State Operand (enc_func_state)  ---
const CEC_OP_ENC_FUNC_STATE_EXT_CON_NOT_SUPPORTED: u8 = 0;
const CEC_OP_ENC_FUNC_STATE_EXT_CON_INACTIVE: u8 = 1;
const CEC_OP_ENC_FUNC_STATE_EXT_CON_ACTIVE: u8 = 2;
// ---  CDC Error Code Operand (cdc_errcode)  ---
const CEC_OP_CDC_ERROR_CODE_NONE: u8 = 0;
const CEC_OP_CDC_ERROR_CODE_CAP_UNSUPPORTED: u8 = 1;
const CEC_OP_CDC_ERROR_CODE_WRONG_STATE: u8 = 2;
const CEC_OP_CDC_ERROR_CODE_OTHER: u8 = 3;
// ---  HEC Support Operand (hec_support)  ---
const CEC_OP_HEC_SUPPORT_NO: u8 = 0;
const CEC_OP_HEC_SUPPORT_YES: u8 = 1;
// ---  HEC Activation Operand (hec_activation)  ---
const CEC_OP_HEC_ACTIVATION_ON: u8 = 0;
const CEC_OP_HEC_ACTIVATION_OFF: u8 = 1;

// ---  HEC Set State Operand (hec_set_state)  ---
const CEC_OP_HEC_SET_STATE_DEACTIVATE: u8 = 0;
const CEC_OP_HEC_SET_STATE_ACTIVATE: u8 = 1;
const CEC_OP_HPD_STATE_CP_EDID_DISABLE: u8 = 0;
const CEC_OP_HPD_STATE_CP_EDID_ENABLE: u8 = 1;
const CEC_OP_HPD_STATE_CP_EDID_DISABLE_ENABLE: u8 = 2;
const CEC_OP_HPD_STATE_EDID_DISABLE: u8 = 3;
const CEC_OP_HPD_STATE_EDID_ENABLE: u8 = 4;
const CEC_OP_HPD_STATE_EDID_DISABLE_ENABLE: u8 = 5;
// ---  HPD Error Code Operand (hpd_error)  ---
const CEC_OP_HPD_ERROR_NONE: u8 = 0;
const CEC_OP_HPD_ERROR_INITIATOR_NOT_CAPABLE: u8 = 1;
const CEC_OP_HPD_ERROR_INITIATOR_WRONG_STATE: u8 = 2;
const CEC_OP_HPD_ERROR_OTHER: u8 = 3;
const CEC_OP_HPD_ERROR_NONE_NO_VIDEO: u8 = 4;
*/
