use crate::{
    CecCaps, CecEvent, CecLogAddrs, CecLogicalAddress, CecModeFollower, CecModeInitiator, CecMsg,
    CecOpcode, CecPhysicalAddress,
};
use nix::libc::O_NONBLOCK;
use std::fs::OpenOptions;
use std::io::Result;
use std::os::unix::fs::OpenOptionsExt;
use tokio::io::{unix::AsyncFd, Interest};

pub struct AsyncCec(AsyncFd<super::CecDevice>);

impl AsyncCec {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        /* When the O_NONBLOCK flag is given, the CEC_RECEIVE and CEC_DQEVENT() ioctls
         * will return the EAGAIN error code when no message or event is available,
         * and ioctls CEC_TRANSMIT, CEC_ADAP_S_PHYS_ADDR and CEC_ADAP_S_LOG_ADDRS all return 0.
         */
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open(path)?;
        let a = AsyncFd::with_interest(
            super::CecDevice(f),
            Interest::READABLE | Interest::WRITABLE | Interest::PRIORITY,
        )?;
        Ok(Self(a))
    }
    pub async fn rec(&self) -> Result<CecMsg> {
        self.0
            .async_io(Interest::READABLE, |inner| inner.rec())
            .await
    }
    pub async fn get_event(&self) -> Result<CecEvent> {
        self.0
            .async_io(Interest::PRIORITY, |inner| inner.get_event())
            .await
    }
    pub async fn transmit(
        &self,
        from: CecLogicalAddress,
        to: CecLogicalAddress,
        opcode: CecOpcode,
    ) -> Result<()> {
        self.0
            .async_io(Interest::WRITABLE, |inner| inner.transmit(from, to, opcode))
            .await
    }
    pub async fn transmit_data(
        &self,
        from: CecLogicalAddress,
        to: CecLogicalAddress,
        opcode: CecOpcode,
        data: &[u8],
    ) -> Result<()> {
        self.0
            .async_io(Interest::WRITABLE, |inner| {
                inner.transmit_data(from, to, opcode, data)
            })
            .await
    }
    pub fn get_capas(&self) -> Result<CecCaps> {
        self.0.get_ref().get_capas()
    }
    pub fn get_mode(&self) -> Result<(CecModeInitiator, CecModeFollower)> {
        self.0.get_ref().get_mode()
    }
    pub fn get_log(&self) -> Result<CecLogAddrs> {
        self.0.get_ref().get_log()
    }
    pub fn get_phys(&self) -> Result<CecPhysicalAddress> {
        self.0.get_ref().get_phys()
    }
    pub fn set_log(&self, log: CecLogAddrs) -> Result<()> {
        self.0.get_ref().set_log(log)
    }
    pub fn set_mode(&self, initiator: CecModeInitiator, follower: CecModeFollower) -> Result<()> {
        self.0.get_ref().set_mode(initiator, follower)
    }
    pub fn set_phys(&self, addr: CecPhysicalAddress) -> Result<()> {
        self.0.get_ref().set_phys(addr)
    }
}
