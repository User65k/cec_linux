[![Project Status: Active – The project has reached a stable, usable state and is being actively developed.](https://www.repostatus.org/badges/latest/active.svg)](https://www.repostatus.org/#active)
[![GitHub](https://img.shields.io/github/license/User65k/cec_linux)](./LICENSE)
![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/User65k/cec_linux/rust.yml)
[![crates.io](https://img.shields.io/crates/v/cec_linux.svg)](https://crates.io/crates/cec_linux)
[![Released API docs](https://docs.rs/cec_linux/badge.svg)](https://docs.rs/cec_linux)

A pure rust library to use the [CEC linux API](https://www.kernel.org/doc/html/v4.9/media/uapi/cec/cec-api.html) (as found on Raspberry Pis with bookworm)

# Background

While libcec and thus [cec-rs](https://crates.io/crates/cec-rs) also works with the linux driver,
I noticed after upgrading from bullseye to bookworm that some messages are only visible to me in monitor mode.
So they are missing in cec-rs as well as the old firmware driver is gone in bookworm.
