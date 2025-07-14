use std::fs::File;
use std::io::prelude::*;
use infocom::disassembler::disassemble_range;

fn main() -> std::io::Result<()> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    // Disassemble routine at 0x3770
    match disassemble_range(&game_data, 0x3770, 0x3800) {
        Ok(disasm) => println\!("{}", disasm),
        Err(e) => eprintln\!("Disassembly error: {}", e),
    }

    Ok(())
}
EOF < /dev/null