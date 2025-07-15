# Zork I Routine Address Table

This table contains known routine addresses and their names discovered through debugging.

## Known Routines

| Address | Name | Description |
|---------|------|-------------|
| 0x4f05 | MAIN | Main entry point - initializes game and calls V-VERSION |
| 0x50a8 | (Unknown) | Previously thought to be PERFORM |
| 0x6ee0 | V-VERSION | Prints copyright/version info (called from MAIN at 0x4f82) |
| 0x51f0 | GOTO | Changes current location (G0/HERE) |
| 0x552a | MAIN-LOOP | Main loop that calls PERFORM and other routines |
| 0x577c | PERFORM | Main command processing routine - checks property 17 |
| 0x5880 | PARSER | Command parser - called by MAIN-LOOP |
| 0x590c | INPUT-LOOP | Main input loop (contains SREAD) |
| 0x5c40 | (Unknown-5c40) | Previously thought to be PARSER |
| 0x6f76 | V-WALK | Walk/movement verb handler |
| 0x7086 | LIT? | Check if location is lit |
| 0x7e04 | DESCRIBE-ROOM | Room description routine |
| 0x8c9a | DESCRIBE-OBJECTS | Describe objects in location |

## Problem Areas

| Address | Issue |
|---------|-------|
| 0x58fa | Buggy get_parent instruction that overwrites G0 |

## Global Variables

| Variable | Name | Description |
|----------|------|-------------|
| G0 (0x10) | HERE | Current location/room |
| G56 | PRSO | Direct object |
| G57 | PRSI | Indirect object |
| G72 | ACT | Current action/verb |
| G76 | P-WALK-DIR | Walking direction (e.g., 0x1d=29 for 'w') |
| G78 | (Action code) | Used by PERFORM (e.g., 0x89=137 for certain actions) |
| G6f (V7f) | (Actor/Player) | Contains object 4 (ADVENTURER) |

## Important Objects

### Key Locations
| Object # | Name |
|----------|------|
| 180 | West of House |
| 239 | Forest |
| 79 | Behind House |
| 80 | South of House |
| 81 | North of House |
| 72 | Cellar |
| 74 | Clearing |
| 75 | Forest Path |
| 76 | Forest |
| 77 | Forest |
| 78 | Forest |

### Underground Areas
| Object # | Name |
|----------|------|
| 15 | Slide Room |
| 16-19 | Coal Mine |
| 20 | Ladder Bottom |
| 21 | Ladder Top |
| 22 | Smelly Room |
| 23 | Squeaky Room |
| 24 | Mine Entrance |
| 57 | Grating Room |
| 102 | The Troll Room |
| 105 | Torch Room |
| 107 | Round Room |

### Maze Rooms
| Object # | Name |
|----------|------|
| 52-56 | Maze |
| 58-70 | Maze |
| 55, 61, 65, 66, 118 | Dead End |
| 167 | Maze |

### Other Notable Locations
| Object # | Name |
|----------|------|
| 25 | Canyon View |
| 26 | Rocky Ledge |
| 27 | Canyon Bottom |
| 28 | On the Rainbow |
| 29 | Aragain Falls |
| 30 | Shore |
| 32-33 | White Cliffs Beach |
| 88 | Up a Tree |
| 94 | Studio |
| 96 | Engravings Cave |
| 120 | Sandy Beach |
| 124 | Gas Room |
| 126 | Sandy Cave |
| 133 | Dome Room |
| 136 | End of Rainbow |
| 138 | Loud Room |
| 140 | Dam Base |
| 143 | Clearing |
| 148 | Gallery |
| 150, 152 | Mirror Room |
| 154 | Dam Lobby |
| 157 | Machine Room |
| 172 | Reservoir North |

### Special Objects
| Object # | Name |
|----------|------|
| 1 | pair of hands |
| 2 | zorkmid |
| 4 | cretin |
| 5 | you |

## Detailed Routine Information

### 0x552a - MAIN-LOOP
The main game loop that processes commands.
- First calls PARSER (0x5880) to parse the command
- Loads parsed words from G55/G56 buffers
- When G78=0x89 (137), calls PERFORM (0x577c) with arguments (G78, G76)
- For 'w' command: G76=0x1d (29), which matches the data byte from dictionary entry

### 0x577c - PERFORM
The main command processing routine. Handles executing parsed commands.
- Takes parameters: L00=action code (e.g. 0x89), L01=direct object/direction (e.g. 0x1d for 'w')
- Stores parameters: L04=G78, L05=G76, L06=G77
- Checks property 17 (action handler) of object in G6f (variable 0x7f = object 4)
- Falls back to checking parent's property 17
- Continues with other checks if those fail

### 0x5880 - PARSER
The command parser routine.
- Called by MAIN-LOOP at the start of each command cycle
- Parses user input and sets up global variables
- Returns result in G7f

### 0x6ee0 - V-VERSION
Action routine for "version" command.
- Called from MAIN at 0x4f82 during initialization
- Prints "ZORK I: The Great Underground Empire"
- Prints copyright notice
- Prints revision and serial number from header

## Notes

- The bug at 0x58fa executes `get_parent V7f -> V10` which corrupts the current location
- Movement commands fail because G0 is overwritten after every input
- GOTO is never actually called during normal movement commands