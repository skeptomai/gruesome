# Gruesome Z-Machine Platform: Quick Start Guide

**Get started in 5 minutes!** ⚡

## What Is This?

The Gruesome Z-Machine Platform gives you three powerful tools:

1. **🎮 Play classic text adventures** (Zork, AMFV, Trinity)
2. **⚡ Create your own games** using the modern Grue language  
3. **🔍 Analyze and debug** Z-Machine games

## Prerequisites

- **Rust** (install from [rustup.rs](https://rustup.rs/))
- **Git** (to clone the repository)
- **Terminal/Command Line** (basic familiarity)

## Installation (2 minutes)

```bash
# Clone the repository
git clone https://github.com/your-org/gruesome-z-machine
cd gruesome-z-machine

# Build all tools (takes ~2 minutes)
cargo build --release

# Verify installation
cargo test --release
```

**You now have three tools ready to use:**
- `cargo run --release --bin gruesome` - Game interpreter
- `cargo run --release --bin grue-compiler` - Game compiler  
- `cargo run --release --bin gruedasm-txd` - Game disassembler

## 🎮 Playing Games (30 seconds)

### Try Zork I (included!)
```bash
cargo run --release --bin gruesome resources/test/zork1/DATA/ZORK1.DAT
```

You're now playing the original Zork! Try typing:
- `look` - Examine your surroundings
- `inventory` - Check what you're carrying
- `go north` - Move around
- `quit` - Exit the game

### Try a V4 Game (AMFV)
```bash  
cargo run --release --bin gruesome resources/test/amfv/amfv-r79-s851122.z4
```

## ⚡ Creating Your First Game (2 minutes)

### Step 1: Write a Simple Game
Create `my_first_game.grue`:
```grue
// My First Text Adventure!
world {
    room starting_room "The Starting Room" {
        desc: "You are in a cozy room with a doorway to the north."
        exits: {
            north: forest_path
        }
    }
    
    room forest_path "Forest Path" {
        desc: "A winding path through tall trees. The room is to the south."
        exits: {
            south: starting_room  
        }
    }
    
    object magic_lamp "magic lamp" in starting_room {
        takeable: true
        desc: "An ornate brass lamp with mysterious engravings."
    }
}

fn describe_lamp() {
    print("The lamp glows with an inner light!");
}

init {
    print("Welcome to your first adventure!");
    print("Try: look, go north, take lamp, examine lamp");
}
```

### Step 2: Compile Your Game
```bash
# Compile to Z-Machine V3 (classic format)
cargo run --release --bin grue-compiler -- my_first_game.grue

# Or specify a different version:
cargo run --release --bin grue-compiler -- --version v4 my_first_game.grue
cargo run --release --bin grue-compiler -- --version v5 my_first_game.grue
```

### Step 3: Play Your Game!
```bash
cargo run --release --bin gruesome my_first_game.z3
```

**Congratulations!** You've just created and played your own text adventure game! 🎉

## 🔍 Analyzing Games (Advanced)

Want to understand how a Z-Machine game works?

```bash
# Disassemble any Z-Machine file
cargo run --release --bin gruedasm-txd my_first_game.z3 > analysis.txt

# Analyze a classic game  
cargo run --release --bin gruedasm-txd resources/test/zork1/DATA/ZORK1.DAT > zork_analysis.txt
```

## Example Games to Try

The repository includes several example Grue programs:

```bash
# Compile and run examples
cargo run --release --bin grue-compiler -- examples/test_01_basic.grue
cargo run --release --bin gruesome test_01_basic.z3

cargo run --release --bin grue-compiler -- examples/mini_zork.grue  
cargo run --release --bin gruesome mini_zork.z3
```

## Next Steps

### Learn More Grue Programming
- 📖 **[Grue User Guide](GRUE_USER_GUIDE.md)** - Complete language tutorial
- 🏗️ **[Compiler Architecture](Grue_Compiler_Architecture.md)** - How the compiler works
- 📋 **[Implementation Status](IMPLEMENTATION_STATUS.md)** - What features are available

### For Developers
- 🔧 **[Developer Architecture Guide](DEVELOPER_ARCHITECTURE_GUIDE.md)** - Complete platform overview
- 🎮 **[Architecture](ARCHITECTURE.md)** - Interpreter internals
- 🔍 **[Disassembler Design](DISASSEMBLER_DESIGN.md)** - Analysis tool details

### Advanced Features
- **Save/Restore**: Most games support saving your progress
- **Multiple Versions**: Compile to V3 (classic), V4 (enhanced), or V5 (advanced)
- **Cross-Platform**: Works on Windows, macOS, and Linux
- **Performance**: Optimized for speed and compatibility

## Troubleshooting

### Build Issues
```bash
# Clean and rebuild
cargo clean
cargo build --release

# Update Rust
rustup update
```

### Game Issues  
```bash
# Enable debug logging
RUST_LOG=debug cargo run --bin gruesome your_game.z3

# Check the game file
file your_game.z3  # Should show "Infocom game data"
```

### Compilation Issues
```bash
# Check syntax with detailed errors
RUST_LOG=debug cargo run --bin grue-compiler -- your_game.grue
```

## Get Help

- **Documentation**: Check the `/docs` directory for detailed guides
- **Examples**: Look at `/examples` for sample Grue programs  
- **Tests**: See `/tests` for usage examples
- **Issues**: Report bugs in the project issue tracker

## What's Special About This Platform?

✨ **Modern**: Built with Rust for safety and performance  
✨ **Complete**: Interpreter + Compiler + Disassembler in one package  
✨ **Compatible**: Runs original Infocom games perfectly  
✨ **Extensible**: Easy to add new features and opcodes  
✨ **Cross-Platform**: Works everywhere Rust works  
✨ **Well-Tested**: 148+ unit tests ensure reliability  

---

**You're ready to explore the world of interactive fiction!** 🌟

Whether you want to play classic games, create new adventures, or dive deep into Z-Machine internals, the Gruesome platform has everything you need to get started.

*Happy adventuring!* 🗺️⚔️🏰