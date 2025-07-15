# Zork I Routine Address Table

This table contains known routine addresses and their names discovered through debugging.

## Known Routines

| Address | Name | Description |
|---------|------|-------------|
| 0x4f05 | MAIN | Main entry point (skipping intro) |
| 0x4f82 | INTRO | Introduction sequence |
| 0x50a8 | PERFORM | Main action performer |
| 0x51f0 | GOTO | Changes current location (G0/HERE) |
| 0x552a | WEST-HOUSE | West of House room handler |
| 0x5880 | FOREST-ROOM | Forest room handler |
| 0x590c | INPUT-LOOP | Main input loop (contains SREAD) |
| 0x5c40 | PARSER | Command parser |
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
| G76 | P-WALK-DIR | Walking direction |

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

## Notes

- The bug at 0x58fa executes `get_parent V7f -> V10` which corrupts the current location
- Movement commands fail because G0 is overwritten after every input
- GOTO is never actually called during normal movement commands