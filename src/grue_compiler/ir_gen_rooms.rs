// IR Generator - Room Generation
//
// Extracted from ir_generator.rs as part of modularization effort.
// Handles conversion of AST room declarations to IR room structures.

use crate::grue_compiler::error::CompilerError;
use indexmap::IndexMap;

use super::{IrExitTarget, IrGenerator, IrRoom};

impl IrGenerator {
    /// Generate IR room from AST room declaration
    ///
    /// Processes room exits, objects, and handler blocks (on_enter, on_exit, on_look).
    /// Validates exit targets and manages object hierarchy recording.
    pub(super) fn generate_room(
        &mut self,
        room: crate::grue_compiler::ast::RoomDecl,
    ) -> Result<IrRoom, CompilerError> {
        // Room ID should already be pre-registered during first pass
        let room_id = *self.symbol_ids.get(&room.identifier).unwrap_or_else(|| {
            panic!(
                "Room '{}' should have been pre-registered in first pass",
                room.identifier
            )
        });

        // Object numbers should already be assigned during registration pass
        // Don't reassign if already exists (avoids duplicate assignment bug)
        if let Some(&existing_number) = self.object_numbers.get(&room.identifier) {
            log::debug!(
                "IR generate_room: Room '{}' already has object number {} from registration pass",
                room.identifier,
                existing_number
            );
        } else {
            // This should never happen with systematic object numbering
            return Err(CompilerError::CodeGenError(format!(
                "IR generate_room: Room '{}' should have been assigned an object number during systematic assignment but wasn't found",
                room.identifier
            )));
        }

        let mut exits = IndexMap::new();
        log::debug!(
            "IR generate_room: Processing {} exits for room '{}'",
            room.exits.len(),
            room.identifier
        );
        for (direction, target) in room.exits {
            log::debug!("IR generate_room: Exit '{}' -> {:?}", direction, target);
            let ir_target = match target {
                crate::grue_compiler::ast::ExitTarget::Room(room_name) => {
                    // Look up target room IR ID from symbol table
                    let target_room_id = *self.symbol_ids.get(&room_name).unwrap_or(&0);
                    if target_room_id == 0 {
                        return Err(CompilerError::CodeGenError(format!(
                            "Exit from room '{}' references undefined room '{}'",
                            room.identifier, room_name
                        )));
                    }
                    IrExitTarget::Room(target_room_id)
                }
                crate::grue_compiler::ast::ExitTarget::Blocked(message) => {
                    IrExitTarget::Blocked(message)
                }
            };
            exits.insert(direction, ir_target);
        }

        // Process room objects FIRST - add them to symbol_ids for identifier resolution
        // This must happen before processing handlers that might reference these objects
        log::debug!(
            "Processing {} objects for room '{}'",
            room.objects.len(),
            room.identifier
        );

        // Phase 1b: Record object hierarchy in room_objects mapping
        let mut room_object_infos = Vec::new();
        for obj in &room.objects {
            self.register_object_and_nested(obj)?;

            // Extract object hierarchy and add to room mapping
            let object_info = self.extract_object_hierarchy(obj);
            room_object_infos.push(object_info);
        }

        // Store the complete object hierarchy for this room
        if !room_object_infos.is_empty() {
            self.room_objects
                .insert(room.identifier.clone(), room_object_infos);
            log::debug!(
                "Phase 1b: Recorded {} object hierarchies for room '{}'",
                self.room_objects[&room.identifier].len(),
                room.identifier
            );
        }

        // Now process handlers - objects are available for reference
        let on_enter = if let Some(block) = room.on_enter {
            Some(self.generate_block(block)?)
        } else {
            None
        };

        let on_exit = if let Some(block) = room.on_exit {
            Some(self.generate_block(block)?)
        } else {
            None
        };

        let on_look = if let Some(block) = room.on_look {
            Some(self.generate_block(block)?)
        } else {
            None
        };

        Ok(IrRoom {
            id: room_id,
            name: room.identifier,
            display_name: room.display_name,
            description: room.description,
            exits,
            on_enter,
            on_exit,
            on_look,
        })
    }
}
