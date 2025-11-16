# Mini Zork - Detailed Function List

## Source Code Location
- `/Users/cb/Projects/infocom-testing-old/infocom/examples/mini_zork.grue`
- Lines 1-642

## Complete Function Inventory

### USER-DEFINED FUNCTIONS (Listed in source order)

#### Score/Stats Functions
1. **handle_score()** [lines 264-268]
   - Parameters: 0
   - Returns: void
   - Purpose: Display player score

2. **handle_moves()** [lines 270-275]
   - Parameters: 0
   - Returns: void
   - Purpose: Display move count

#### Gameplay Core Functions
3. **look_around()** [lines 277-286]
   - Parameters: 0
   - Returns: void
   - Purpose: Display room and call on_look handler

4. **examine(obj)** [lines 288-319]
   - Parameters: 1 (object)
   - Returns: void
   - Purpose: Examine an object with special handling for mailbox and nest

#### Container Interaction
5. **handle_open(obj)** [lines 321-341]
   - Parameters: 1 (object)
   - Returns: void
   - Purpose: Open an object

6. **handle_close(obj)** [lines 343-360]
   - Parameters: 1 (object)
   - Returns: void
   - Purpose: Close an object

#### Object Manipulation Functions (with overloads)
7. **handle_take(obj)** - GENERIC [lines 362-388]
   - Parameters: 1 (object)
   - Returns: void
   - Purpose: Generic take command

8. **handle_take(leaflet)** - SPECIALIZED [lines 390-407]
   - Parameters: 1 (leaflet object)
   - Returns: void
   - Purpose: Take leaflet with score bonus (+2 points)

9. **handle_take(egg)** - SPECIALIZED [lines 409-425]
   - Parameters: 1 (egg object)
   - Returns: void
   - Purpose: Take egg with score bonus (+5 points)

10. **handle_drop(obj)** - GENERIC [lines 427-435]
    - Parameters: 1 (object)
    - Returns: void
    - Purpose: Generic drop command

11. **handle_drop(egg)** - SPECIALIZED [lines 437-445]
    - Parameters: 1 (egg object)
    - Returns: void
    - Purpose: Drop egg with special message

#### Inventory Management
12. **show_inventory()** [lines 447-457]
    - Parameters: 0
    - Returns: void
    - Purpose: Display player inventory

#### Navigation
13. **handle_go(direction)** [lines 459-492]
    - Parameters: 1 (direction string or object)
    - Returns: void
    - Purpose: Move player in specified direction
    - Calls: look_around() on successful move

#### Special Climbing (with overloads)
14. **handle_climb(obj)** - GENERIC [lines 494-500]
    - Parameters: 1 (object)
    - Returns: void
    - Purpose: Generic climb command

15. **handle_climb(tree)** - SPECIALIZED [lines 502-508]
    - Parameters: 1 (tree object)
    - Returns: void
    - Purpose: Climb tree (calls handle_go("up"))

#### Reading (with overloads)
16. **handle_read(obj)** - GENERIC [lines 510-517]
    - Parameters: 1 (object)
    - Returns: void
    - Purpose: Generic read command (not readable)

17. **handle_read(leaflet)** - SPECIALIZED [lines 519-526]
    - Parameters: 1 (leaflet object)
    - Returns: void
    - Purpose: Read leaflet (display its description)

#### Quit System
18. **handle_quit()** [lines 528-531]
    - Parameters: 0
    - Returns: void
    - Purpose: Initiate quit dialog

19. **handle_yes()** [lines 533-540]
    - Parameters: 0
    - Returns: void
    - Purpose: Handle "yes" response to quit prompt

20. **handle_no()** [lines 542-549]
    - Parameters: 0
    - Returns: void
    - Purpose: Handle "no" response to quit prompt

21. **clear_quit_state()** [lines 551-553]
    - Parameters: 0
    - Returns: void
    - Purpose: Clear quit pending flag (called at start of new commands)

#### Utility Functions
22. **player_can_see(obj)** [lines 555-580]
    - Parameters: 1 (object)
    - Returns: bool
    - Purpose: Determine if player can see object
    - Logic: Checks location visibility and container status

23. **list_objects(location)** [lines 582-599]
    - Parameters: 1 (location)
    - Returns: void
    - Purpose: List all objects in a location

24. **list_contents(container)** [lines 601-606]
    - Parameters: 1 (container)
    - Returns: void
    - Purpose: List contents of a container

25. **take_all()** [lines 608-623]
    - Parameters: 0
    - Returns: void
    - Purpose: Take all takeable objects in current location

#### Initialization
26. **init()** [lines 625-642]
    - Parameters: 0
    - Returns: void
    - Purpose: Game initialization (entry point)
    - Sets: Initial player location, score, moves, banner

### COMPILER-GENERATED BUILTIN FUNCTIONS (5)

1. **get_exit(location, direction)** 
   - Purpose: Get exit from location in specified direction
   - Generated in: codegen.rs generate_builtin_functions()
   - Implementation: Complex Z-Machine routine with exit traversal

2. **print_num(number)**
   - Purpose: Print integer in decimal
   - Generated in: codegen.rs generate_builtin_functions()
   - Implementation: Uses Z-Machine print_num opcode (VAR:230/6)

3. **add_score(points)**
   - Purpose: Add points to player score
   - Generated in: codegen.rs generate_builtin_functions()
   - Implementation: Loads G17 (score), adds value, stores back

4. **subtract_score(points)**
   - Purpose: Subtract points from player score
   - Generated in: codegen.rs generate_builtin_functions()
   - Implementation: Loads G17 (score), subtracts value, stores back

5. **word_to_number(word)**
   - Purpose: Convert word string to number
   - Generated in: codegen.rs generate_builtin_functions()
   - Implementation: Number word recognition and conversion

### ROOM DEFINITIONS (8)

1. **west_of_house** [lines 23-53]
   - Description: "West of House"
   - Contains: mailbox (with leaflet inside)
   - Exits: north, south, east (blocked)
   - Event: on_enter (welcome message on first visit)

2. **north_of_house** [lines 55-61]
   - Description: "North of House"
   - Exits: south, north

3. **south_of_house** [lines 63-69]
   - Description: "South of House"
   - Exits: north, east

4. **behind_house** [lines 71-91]
   - Description: "Behind House"
   - Contains: window object
   - Exits: west, east, south
   - Event: on_look (shows kitchen view if window is open)

5. **forest_path** [lines 93-109]
   - Description: "Forest Path"
   - Contains: tree object
   - Exits: south, east, up

6. **up_a_tree** [lines 111-135]
   - Description: "Up a Tree"
   - Contains: nest (with egg inside)
   - Exits: down

7. **forest** [lines 137-146]
   - Description: "Forest"
   - Exits: north, south, west

8. **clearing** [lines 148-155]
   - Description: "Forest Clearing"
   - Exits: north

### OBJECT DEFINITIONS (9 total)

Within west_of_house:
1. **mailbox** - Container, openable
   - Contains: leaflet

Within mailbox:
2. **leaflet** - Takeable item (valuable for score)

Within forest_path:
3. **tree** - Non-takeable object

Within behind_house:
4. **window** - Openable, non-takeable

Within up_a_tree:
5. **nest** - Container, open
   - Contains: egg

Within nest:
6. **egg** - Openable, container, takeable (valuable for score)

Player/System objects (implicitly):
7. **player** - Main character object (G16 in Z-Machine)

Global variables:
8-9. **Player properties**: score, moves, location, quit_pending

## Grammar Actions

The grammar section (lines 158-262) defines 21 verb handlers that invoke the above functions:

- "score" → handle_score()
- "moves" → handle_moves()
- "look" (3 patterns) → look_around() or examine()
- "examine" → examine()
- "open" → handle_open()
- "close" → handle_close()
- "take" (2 patterns) → handle_take() or take_all()
- "drop" → handle_drop()
- "inventory" → show_inventory()
- "go" → handle_go()
- "north" → handle_go("north")
- "south" → handle_go("south")
- "east" → handle_go("east")
- "west" → handle_go("west")
- "up" → handle_go("up")
- "down" → handle_go("down")
- "climb" → handle_climb()
- "read" → handle_read()
- "quit"/"exit"/"q" → handle_quit()
- "yes"/"y" → handle_yes()
- "no"/"n" → handle_no()

## Summary Statistics

- **User-Defined Functions**: 26 (counting each overload separately)
- **Builtin Functions**: 5
- **Rooms**: 8
- **Objects**: 9 (not counting player or implicit system objects)
- **Grammar Verbs**: 21 (multiple patterns for some)

## Total Expected Routines in Compiled Output

When compiling mini_zork.grue to Z3:

**Minimum**: 31 routines
- 26 user functions
- 5 builtins
- Typically does NOT generate separate init routine (merged with main)

**Typical**: 32-35 routines
- 26 user functions
- 5 builtins
- Optional: 1-4 grammar/system helper functions

**Maximum**: 40-50 routines
- If grammar dispatch functions are created
- If room initialization generates separate routines
- If special object handlers are compiled

The actual count depends on compiler optimization and how overloaded functions are handled.
