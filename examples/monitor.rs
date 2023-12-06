use cec_linux::*;

fn main() -> std::io::Result<()> {
    let cec = CecDevice::open("/dev/cec0")?;
    let capas = cec.get_capas();
    println!("capas  {:?}", capas);
    cec.set_mode(CecModeInitiator::None, CecModeFollower::Monitor)?;

    loop {
        let f = cec.poll(
            PollFlags::POLLIN | PollFlags::POLLRDNORM | PollFlags::POLLPRI,
            -1,
        )?;

        if f.intersects(PollFlags::POLLPRI) {
            let evt = cec.get_event()?;
            println!("evt {:x?}", evt);
        }
        if f.contains(PollFlags::POLLIN | PollFlags::POLLRDNORM) {
            let msg = cec.rec()?;

            if msg.is_ok() {
                match (msg.initiator(), msg.destination(), msg.opcode()) {
                    (Ok(i), Ok(d), Some(Ok(o))) => {
                        println!(
                            "msg {:?}->{:?} {:?} {:x?}",
                            i,
                            d,
                            o,
                            msg.parameters()
                        );
                    }
                    _ => println!("msg {:x?}", &msg.msg[..msg.len as usize]),
                }
            } else {
                println!("msg {:x?}", msg);
            }
        }
    }
}
