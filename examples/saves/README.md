# Example Save Files

These are example save files for Zork I that can be used for testing or to jump to specific points in the game.

## Available Save Files

### in_house_at_trap_door.sav
- **Location**: Inside the white house, at the trap door
- **Progress**: Early game, house has been entered
- **Useful for**: Testing underground areas, cellar access

### at_dam.sav  
- **Location**: At the Flood Control Dam #3
- **Progress**: Mid-game, has explored some underground areas
- **Useful for**: Testing dam mechanics, reservoir areas

## Using Save Files

To restore a save file:

1. Start the game: `cargo run --bin gruesome resources/test/zork1/DATA/ZORK1.DAT`
2. At the prompt, type: `restore`
3. When prompted for filename, enter the path: `examples/saves/in_house_at_trap_door.sav`

## Save File Format

These files use the Quetzal save format with XOR-RLE compression. They contain:
- Complete Z-Machine state (stack, variables, PC)
- Dynamic memory differences from initial state
- Header information for validation

See the Z-Machine specification for details on the Quetzal format.