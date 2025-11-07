# Grue Language Support for VS Code

Syntax highlighting and language support for the Grue interactive fiction programming language.

## Features

- **Comprehensive syntax highlighting** for all Grue language constructs
- **Smart bracket matching** for `{}`, `[]`, and `()`
- **Auto-indentation** for nested blocks
- **Comment toggling** with `Ctrl+/` (Line comments with `//`)
- **Auto-closing pairs** for brackets and quotes

## Supported Syntax

### Keywords
- **Declarations**: `world`, `room`, `object`, `grammar`, `verb`, `fn`, `init`
- **Properties**: `contains`, `names`, `desc`, `properties`, `exits`
- **Events**: `on_enter`, `on_exit`, `on_look`
- **Control Flow**: `if`, `else`, `while`, `for`, `in`, `return`, `let`, `var`
- **Built-ins**: `print`, `print_ret`, `new_line`, `move`, `player`, `location`, `mode`

### Literals
- **Strings**: `"Hello World"` with escape sequences (`\n`, `\t`, `\"`, etc.)
- **Numbers**: `42`, `100`
- **Booleans**: `true`, `false`
- **Parameters**: `$noun`, `$2`, `$object`

### Operators
- **Assignment**: `=`
- **Comparison**: `==`, `!=`, `<`, `<=`, `>`, `>=`
- **Arithmetic**: `+`, `-`, `*`, `/`, `%`
- **Logical**: `&&`, `||`, `!`
- **Ternary**: `?`
- **Safe Access**: `?.`
- **Arrow**: `=>`

## Installation

1. Copy this extension folder to your VS Code extensions directory:
   - **Windows**: `%USERPROFILE%\.vscode\extensions\`
   - **macOS**: `~/.vscode/extensions/`
   - **Linux**: `~/.vscode/extensions/`

2. Reload VS Code or run `Developer: Reload Window`

3. Open any `.grue` file to see syntax highlighting

## Sample Code

```grue
// Mini Zork - Sample Grue Game
world {
    room west_of_house "West of House" {
        desc: "You are standing in an open field west of a white house."

        object mailbox {
            names: ["mailbox", "box"]
            desc: "a small mailbox"
            openable: true
            open: false
        }

        exits: {
            north: north_of_house,
            east: blocked("The door is boarded.")
        }
    }
}

grammar {
    verb "look" {
        pattern: "look at $object" => examine($object)
        pattern: "l $object" => examine($object)
    }
}
```

## About Grue

Grue is a domain-specific language for creating interactive fiction games that compile to Z-Machine bytecode, compatible with classic Infocom interpreters.

**Project**: [gruesome](https://github.com/skeptomai/gruesome)

## License

MIT License - See the main project repository for details.