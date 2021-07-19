#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
compile_error!("this crate cannot be built for a target other than windows i686");

type ID = i32;
#[path = "../../gm8emulator/src/game/external/dll.rs"]
mod dll;
#[path = "../../gm8emulator/src/game/external/state.rs"]
mod state;
#[path = "../../gm8emulator/src/game/external/win32.rs"]
mod win32;

use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::{env, io::{self, Read, Write}};

fn main() -> io::Result<()> {
    let mut externals = win32::NativeExternals::new().unwrap();
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    eprintln!("starting dll compatibility layer\n  > server: \"{}\"", env::args().next().unwrap());

    let mut message = Vec::with_capacity(1024);
    loop {
        message.clear();

        let length = stdin.read_u32::<LE>()? as usize;
        unsafe { message.set_len(length) };
        stdin.read_exact(message.as_mut_slice())?;

        macro_rules! respond {
            ($res:expr) => {{
                let result = $res;
                message.clear();
                bincode::serialize_into(&mut message, &result).expect("failed to serialize message (server)");
                assert!(message.len() <= u32::max_value() as usize);
                stdout.write_u32::<LE>(message.len() as u32)?;
                stdout.write_all(&message[..])?;
                stdout.flush()?;
            }};
        }

        match bincode::deserialize::<dll::Wow64Message>(&message)
            .expect("failed to deserialize message (server)")
        {
            dll::Wow64Message::Call(id, args)
                => respond!(externals.call(id, &args)),
            dll::Wow64Message::Define(dll, sym, cconv, args, ret)
                => respond!(externals.define(&dll, &sym, cconv, &args, ret)),
            dll::Wow64Message::DefineDummy(dll, sym, dummy, argc)
                => respond!(externals.define_dummy(&dll, &sym, dummy, argc)),
            dll::Wow64Message::Free(dll)
                => respond!(externals.free(&dll)),
            dll::Wow64Message::GetNextId
                => respond!(externals.ss_id()),
            dll::Wow64Message::SetNextId(id)
                => respond!(externals.ss_set_id(id)),
            dll::Wow64Message::QueryDefs
                => respond!(externals.ss_query_defs()),
            dll::Wow64Message::Stop => {
                respond!(Result::<(), String>::Ok(()));
                break Ok(())
            },
        }
    }
}
