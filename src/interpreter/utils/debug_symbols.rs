//! Debug symbol names for Zork I
//!
//! This module provides human-readable names for various game elements:
//! - Routine addresses and their function names
//! - Global variable names
//! - Object names (rooms, items, etc.)

use std::collections::HashMap;

/// Known routine addresses and their names for Zork I
pub struct RoutineNames {
    names: HashMap<u32, &'static str>,
}

impl Default for RoutineNames {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutineNames {
    pub fn new() -> Self {
        let mut names = HashMap::new();

        // Main routines
        names.insert(0x4f05, "MAIN");
        names.insert(0x4fd9, "NOT-HERE-OBJECT-F");
        names.insert(0x508e, "NOT-HERE-PRINT");
        names.insert(0x50a8, "(Unknown-50a8)");
        names.insert(0x6ee0, "V-VERSION");
        names.insert(0x51f0, "GOTO");
        names.insert(0x552a, "MAIN-LOOP");
        names.insert(0x577c, "PERFORM");
        names.insert(0x5880, "PARSER-MAIN");
        names.insert(0x590c, "INPUT-LOOP");
        names.insert(0x5c40, "PARSER");
        names.insert(0x6301, "BUFFER-PRINT");
        names.insert(0x6f76, "V-WALK");
        names.insert(0x7086, "LIT?");
        names.insert(0x7e04, "DESCRIBE-ROOM");
        names.insert(0x8c9a, "DESCRIBE-OBJECTS");
        names.insert(0x5fda, "WORD-PRINT");

        RoutineNames { names }
    }

    /// Get the name of a routine at the given address
    pub fn get_name(&self, addr: u32) -> Option<&'static str> {
        self.names.get(&addr).copied()
    }

    /// Get the routine that contains the given address
    pub fn get_routine_containing(&self, addr: u32) -> Option<(u32, &'static str)> {
        // Find the highest routine address that's less than or equal to addr
        let mut best_match: Option<(u32, &'static str)> = None;

        for (&routine_addr, &name) in &self.names {
            if routine_addr <= addr {
                if let Some((best_addr, _)) = best_match {
                    if routine_addr > best_addr {
                        best_match = Some((routine_addr, name));
                    }
                } else {
                    best_match = Some((routine_addr, name));
                }
            }
        }

        // Only return if the address is reasonably close (within 1000 bytes)
        if let Some((routine_addr, name)) = best_match {
            if addr - routine_addr < 1000 {
                return Some((routine_addr, name));
            }
        }

        None
    }

    /// Format an address with its name if known
    pub fn format_address(&self, addr: u32) -> String {
        if let Some(name) = self.get_name(addr) {
            format!("{addr:04x} ({name})")
        } else if let Some((routine_addr, name)) = self.get_routine_containing(addr) {
            if routine_addr == addr {
                format!("{addr:04x} ({name})")
            } else {
                format!("{addr:04x} (in {name})")
            }
        } else {
            format!("{addr:04x}")
        }
    }
}

/// Global variable names
pub fn get_global_name(var_num: u8) -> Option<&'static str> {
    match var_num {
        0x10 => Some("HERE"),           // G00 -> V10
        0x48 => Some("PRSO"),           // G38 -> V48
        0x49 => Some("PRSI"),           // G39 -> V49
        0x58 => Some("ACT"),            // G48 -> V58
        0x5c => Some("P-WALK-DIR"),     // G4c -> V5c
        0x5e => Some("(Action-code)"),  // G4e -> V5e
        0x7f => Some("(Actor/Player)"), // G6f -> V7f
        _ => None,
    }
}

/// Format a global variable with its name if known
pub fn format_global(var_num: u8) -> String {
    if let Some(name) = get_global_name(var_num) {
        format!("G{} ({})", var_num - 0x10, name)
    } else {
        format!("G{}", var_num - 0x10)
    }
}

/// Known object names
pub fn get_object_name(obj_num: u16) -> Option<&'static str> {
    match obj_num {
        1 => Some("pair of hands"),
        2 => Some("zorkmid"),
        4 => Some("cretin"),
        5 => Some("you"),
        15 => Some("Slide Room"),
        16 => Some("Coal Mine"),
        17 => Some("Coal Mine"),
        18 => Some("Coal Mine"),
        19 => Some("Coal Mine"),
        20 => Some("Ladder Bottom"),
        21 => Some("Ladder Top"),
        22 => Some("Smelly Room"),
        23 => Some("Squeaky Room"),
        24 => Some("Mine Entrance"),
        25 => Some("Canyon View"),
        26 => Some("Rocky Ledge"),
        27 => Some("Canyon Bottom"),
        28 => Some("On the Rainbow"),
        29 => Some("Aragain Falls"),
        30 => Some("Shore"),
        32 => Some("White Cliffs Beach"),
        33 => Some("White Cliffs Beach"),
        37 => Some("Chasm"),
        38 => Some("North-South Passage"),
        39 => Some("Damp Cave"),
        40 => Some("Deep Canyon"),
        41 => Some("East-West Passage"),
        42 => Some("Twisting Passage"),
        43 => Some("Winding Passage"),
        44 => Some("Narrow Passage"),
        45 => Some("Cold Passage"),
        46 => Some("Cave"),
        47 => Some("Cave"),
        49 => Some("Stream View"),
        50 => Some("Reservoir South"),
        51 => Some("Strange Passage"),
        52 => Some("Maze"),
        53 => Some("Maze"),
        54 => Some("Maze"),
        55 => Some("Dead End"),
        56 => Some("Maze"),
        57 => Some("Grating Room"),
        58 => Some("Maze"),
        59 => Some("Maze"),
        60 => Some("Maze"),
        61 => Some("Dead End"),
        62 => Some("Maze"),
        63 => Some("Maze"),
        64 => Some("Maze"),
        65 => Some("Dead End"),
        66 => Some("Dead End"),
        67 => Some("Maze"),
        68 => Some("Maze"),
        69 => Some("Maze"),
        70 => Some("Maze"),
        71 => Some("East of Chasm"),
        72 => Some("Cellar"),
        74 => Some("Clearing"),
        75 => Some("Forest Path"),
        76 => Some("Forest"),
        77 => Some("Forest"),
        78 => Some("Forest"),
        79 => Some("Behind House"),
        80 => Some("South of House"),
        81 => Some("North of House"),
        88 => Some("Up a Tree"),
        94 => Some("Studio"),
        96 => Some("Engravings Cave"),
        102 => Some("The Troll Room"),
        105 => Some("Torch Room"),
        107 => Some("Round Room"),
        118 => Some("Dead End"),
        120 => Some("Sandy Beach"),
        124 => Some("Gas Room"),
        126 => Some("Sandy Cave"),
        133 => Some("Dome Room"),
        136 => Some("End of Rainbow"),
        138 => Some("Loud Room"),
        140 => Some("Dam Base"),
        143 => Some("Clearing"),
        148 => Some("Gallery"),
        150 => Some("Mirror Room"),
        152 => Some("Mirror Room"),
        154 => Some("Dam Lobby"),
        157 => Some("Machine Room"),
        167 => Some("Maze"),
        172 => Some("Reservoir North"),
        180 => Some("West of House"),
        239 => Some("Forest"),
        _ => None,
    }
}

/// Format an object number with its name if known
pub fn format_object(obj_num: u16) -> String {
    if let Some(name) = get_object_name(obj_num) {
        format!("{obj_num} ({name})")
    } else {
        format!("{obj_num}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routine_names() {
        let names = RoutineNames::new();
        assert_eq!(names.get_name(0x5c40), Some("PARSER"));
        assert_eq!(names.get_name(0x1234), None);
        assert_eq!(names.format_address(0x51f0), "51f0 (GOTO)");
        assert_eq!(names.format_address(0x1234), "1234");
    }

    #[test]
    fn test_global_names() {
        assert_eq!(get_global_name(0x10), Some("HERE"));
        assert_eq!(format_global(0x10), "G0 (HERE)");
        assert_eq!(format_global(0x20), "G16");
    }
}
