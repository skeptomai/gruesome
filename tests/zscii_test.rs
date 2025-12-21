// Test ZSCII character decoding (extended character set)
use gruesome::interpreter::text::text::decode_string;

#[test]
fn test_zscii_extended_characters() {
    let mut memory = vec![0u8; 1024];
    let abbrev_table = 0x100;
    let addr = 0x200;

    // Test ZSCII 155 (ä - a-umlaut)
    // ZSCII 155 = 0b10011011 = high 5 bits: 00100 (4), low 5 bits: 11011 (27)
    // Encoding: Shift to A2 (5), ZSCII escape (6), high (4), low (27)
    // Word 1: [5, 6, 4]
    // Word 2: [27, 0, 0] with end bit

    let word1 = (5 << 10) | (6 << 5) | 4;
    let word2 = 0x8000 | (27 << 10);

    memory[addr] = (word1 >> 8) as u8;
    memory[addr + 1] = (word1 & 0xFF) as u8;
    memory[addr + 2] = (word2 >> 8) as u8;
    memory[addr + 3] = (word2 & 0xFF) as u8;

    let (decoded, _) = decode_string(&memory, addr, abbrev_table)
        .expect("Should decode ZSCII 155");

    assert_eq!(decoded.trim(), "ä", "ZSCII 155 should decode to ä (a-umlaut)");
}

#[test]
fn test_zscii_inverted_question_mark() {
    let mut memory = vec![0u8; 1024];
    let abbrev_table = 0x100;
    let addr = 0x200;

    // Test ZSCII 223 (¿ - inverted question mark)
    // ZSCII 223 = 0b11011111 = high 5 bits: 00110 (6), low 5 bits: 11111 (31)
    // Encoding: Shift to A2 (5), ZSCII escape (6), high (6), low (31)

    let word1 = (5 << 10) | (6 << 5) | 6;
    let word2 = 0x8000 | (31 << 10);

    memory[addr] = (word1 >> 8) as u8;
    memory[addr + 1] = (word1 & 0xFF) as u8;
    memory[addr + 2] = (word2 >> 8) as u8;
    memory[addr + 3] = (word2 & 0xFF) as u8;

    let (decoded, _) = decode_string(&memory, addr, abbrev_table)
        .expect("Should decode ZSCII 223");

    assert_eq!(decoded.trim(), "¿", "ZSCII 223 should decode to ¿ (inverted question mark)");
}

#[test]
fn test_zscii_guillemet() {
    let mut memory = vec![0u8; 1024];
    let abbrev_table = 0x100;
    let addr = 0x200;

    // Test ZSCII 162 (» - right guillemet)
    // ZSCII 162 = 0b10100010 = high 5 bits: 00101 (5), low 5 bits: 00010 (2)

    let word1 = (5 << 10) | (6 << 5) | 5;
    let word2 = 0x8000 | (2 << 10);

    memory[addr] = (word1 >> 8) as u8;
    memory[addr + 1] = (word1 & 0xFF) as u8;
    memory[addr + 2] = (word2 >> 8) as u8;
    memory[addr + 3] = (word2 & 0xFF) as u8;

    let (decoded, _) = decode_string(&memory, addr, abbrev_table)
        .expect("Should decode ZSCII 162");

    assert_eq!(decoded.trim(), "»", "ZSCII 162 should decode to » (right guillemet)");
}
