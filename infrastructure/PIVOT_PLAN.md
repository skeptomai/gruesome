# Architecture Pivot: Server-Side Execution → Client-Side WASM

**Date**: December 17, 2025
**Objective**: Migrate from server-side game execution to client-side WASM with backend storage API

---

## Architecture Overview

### Client-Side (Browser)
- **WASM interpreter** executes game locally (instant, no latency)
- **Plain HTML/JS UI** for game library and play interface
- **Manual save/load** to backend

### Server-Side (AWS Lambda)
- **Authentication** (Cognito JWT)
- **Game metadata** storage (DynamoDB)
- **Game file** delivery (S3 presigned URLs)
- **Save file** storage (S3 + DynamoDB metadata)

---

## Data Models

### DynamoDB Table: `gruesome-platform`

**Game Metadata Items:**
```
PK: GAME#<game_id>
SK: METADATA

Attributes:
  game_id: string          // "zork1", "mini-zork"
  title: string            // "Zork I: The Great Underground Empire"
  author: string           // "Infocom"
  description: string      // Game description
  version: number          // Z-Machine version (3, 5, 8)
  file_size: number        // Bytes
  s3_key: string           // "zork1.z3"
  created_at: number       // Unix timestamp
```

**Save Metadata Items:**
```
PK: USER#<user_id>
SK: SAVE#<game_id>#<save_name>

Attributes:
  user_id: string          // From JWT sub claim
  game_id: string          // "zork1"
  save_name: string        // User-provided name
  s3_key: string           // "<user_id>/<game_id>/<save_name>.sav"
  file_size: number        // Bytes
  created_at: number       // Unix timestamp
  last_updated: number     // Unix timestamp
```

### S3 Buckets

**gruesome-games/** (public read via presigned URLs)
```
zork1.z3
mini-zork.z3
seastalker.z3
...
```

**gruesome-saves/** (private, user-specific)
```
<user_id>/
  zork1/
    west-of-house.sav
    mailbox-open.sav
  mini-zork/
    checkpoint-1.sav
```

---

## API Specification

### Authentication (Keep Existing)

**POST /api/auth/login**
```json
Request:  { "username": "bob", "password": "secret" }
Response: { "access_token": "jwt...", "expires_in": 3600 }
```

**GET /api/auth/me**
```json
Response: { "user_id": "...", "username": "bob" }
```

---

### Games API (New)

**GET /api/games**
List all available games
```json
Response: {
  "games": [
    {
      "game_id": "zork1",
      "title": "Zork I: The Great Underground Empire",
      "author": "Infocom",
      "description": "...",
      "version": 3,
      "file_size": 82944
    }
  ]
}
```

**GET /api/games/{game_id}**
Get specific game metadata
```json
Response: {
  "game_id": "zork1",
  "title": "Zork I: The Great Underground Empire",
  "author": "Infocom",
  "description": "...",
  "version": 3,
  "file_size": 82944
}
```

**GET /api/games/{game_id}/file**
Get presigned S3 URL to download game file
```json
Response: {
  "download_url": "https://gruesome-games.s3...?presigned",
  "expires_in": 300
}
```

---

### Saves API (New)

**GET /api/saves**
List all saves for authenticated user
```json
Response: {
  "saves": [
    {
      "game_id": "zork1",
      "save_name": "west-of-house",
      "file_size": 12480,
      "created_at": 1734480000,
      "last_updated": 1734480123
    }
  ]
}
```

**GET /api/saves/{game_id}**
List saves for specific game
```json
Response: {
  "game_id": "zork1",
  "saves": [
    {
      "save_name": "west-of-house",
      "file_size": 12480,
      "created_at": 1734480000,
      "last_updated": 1734480123
    }
  ]
}
```

**GET /api/saves/{game_id}/{save_name}**
Get presigned S3 URL to download save file
```json
Response: {
  "download_url": "https://gruesome-saves.s3...?presigned",
  "expires_in": 300
}
```

**POST /api/saves/{game_id}/{save_name}**
Get presigned S3 URL to upload save file
```json
Request: {
  "file_size": 12480  // Optional, for validation
}

Response: {
  "upload_url": "https://gruesome-saves.s3...?presigned",
  "expires_in": 300
}
```

**DELETE /api/saves/{game_id}/{save_name}**
Delete save file
```json
Response: {
  "deleted": true
}
```

---

## Migration Plan

### Phase 1: Backend Cleanup (Priority)

#### Step 1.1: Restructure Lambda Code

**Delete:**
- `lambda/gruesome-api/game/src/game_service.rs` - All execution logic
- `lambda/gruesome-api/game/src/session_service.rs` - Session management
- `src/interpreter/lambda_wrapper.rs` - Server-side interpreter wrapper

**Create:**
- `lambda/gruesome-api/game/src/game_service.rs` - Game metadata CRUD
- `lambda/gruesome-api/game/src/save_service.rs` - Save file CRUD
- `lambda/gruesome-api/game/src/models.rs` - Update request/response types

#### Step 1.2: Update Lambda Dependencies

**Cargo.toml changes:**
```toml
[dependencies]
# Remove gruesome dependency - no longer need interpreter
# gruesome = { path = "../../../../../", default-features = false }

# Keep AWS dependencies
aws-config = "1.1"
aws-sdk-dynamodb = "1.54"
aws-sdk-s3 = "1.62"
lambda_runtime = "0.13"
lambda_http = "0.13"
tokio = "1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4"] }
chrono = "0.4"
```

#### Step 1.3: Implement New Handlers

**File: `lambda/gruesome-api/game/src/handlers.rs`**

Implement:
- `handle_list_games()` - Query DynamoDB for GAME# items
- `handle_get_game()` - Get specific game metadata
- `handle_get_game_file()` - Generate S3 presigned download URL
- `handle_list_saves()` - Query DynamoDB for user's saves
- `handle_get_save()` - Generate S3 presigned download URL
- `handle_create_save()` - Generate S3 presigned upload URL + create DynamoDB metadata
- `handle_delete_save()` - Delete from S3 + DynamoDB

#### Step 1.4: Update API Gateway Routes

**Remove:**
- `POST /api/games/start`
- `POST /api/games/command`

**Add:**
- `GET /api/games`
- `GET /api/games/{game_id}`
- `GET /api/games/{game_id}/file`
- `GET /api/saves`
- `GET /api/saves/{game_id}`
- `GET /api/saves/{game_id}/{save_name}`
- `POST /api/saves/{game_id}/{save_name}`
- `DELETE /api/saves/{game_id}/{save_name}`

**CDK Changes:**
File: `lib/backend-stack.ts`

Update route definitions to new endpoints.

#### Step 1.5: Populate Game Metadata

**Script: `scripts/seed-game-metadata.sh`**

Create script to:
1. Scan S3 `gruesome-games` bucket
2. For each .z3 file, create DynamoDB GAME# item
3. Extract metadata from game file (title, version)

**Manual data entry** for rich metadata (author, description)

#### Step 1.6: Update DynamoDB Schema

**Migration:**
- Current session items (PK=SESSION#) can be deleted
- Add game metadata items (PK=GAME#)
- Save items will be created as users save games

**No table structure changes needed** - same table, different access patterns

#### Step 1.7: Test Backend APIs

Update `scripts/test-game-lambda.sh` → `scripts/test-game-api.sh`:
- Test game listing
- Test game metadata retrieval
- Test presigned URL generation
- Test save CRUD operations

---

### Phase 2: Frontend Implementation

#### Step 2.1: Frontend Project Structure

```
infrastructure/frontend/
├── index.html              # Main entry point
├── css/
│   └── styles.css          # UI styles
├── js/
│   ├── auth.js             # Authentication handling
│   ├── api.js              # API client
│   ├── game-library.js     # Game selection UI
│   └── game-player.js      # Game playing interface
├── wasm/
│   ├── gruesome_wasm.js    # WASM bindings (from build)
│   └── gruesome_wasm_bg.wasm
└── assets/
    └── (game cover images)
```

#### Step 2.2: Authentication UI

**Pages:**
- `login.html` - Login form
- Uses Auth Lambda API
- Stores JWT in localStorage
- Redirects to game library on success

#### Step 2.3: Game Library UI

**Features:**
- Grid/list view of available games
- Fetches from `GET /api/games`
- Shows game metadata (title, author, description)
- Click to play → downloads game file → launches player

#### Step 2.4: WASM Integration

**Game Player:**
- Load WASM module
- Fetch game file via presigned URL
- Initialize interpreter with game bytes
- Render text output
- Handle user input
- Save/Load buttons

**WASM API:**
```javascript
// Already exists in wasm.rs
const game = WasmGame.new(gameBytes);
game.start();

while (true) {
  const output = game.get_output();
  display(output);

  if (game.is_waiting_for_input()) {
    const input = await getUserInput();
    game.execute_command(input);
  }
}
```

#### Step 2.5: Save/Load Implementation

**Save Flow:**
1. User clicks "Save Game"
2. Prompt for save name
3. Get save state: `const saveData = game.save_state()`
4. POST to `/api/saves/{game_id}/{save_name}` → get presigned upload URL
5. PUT save data to presigned URL
6. Update UI with confirmation

**Load Flow:**
1. User clicks "Load Game" → show list of saves
2. GET from `/api/saves/{game_id}` → list saves
3. User selects save
4. GET from `/api/saves/{game_id}/{save_name}` → get presigned download URL
5. Fetch save data from URL
6. `game.restore_state(saveData)`

#### Step 2.6: Deploy Frontend

**CDK Stack** (already exists: `frontend-stack.ts`):
- Upload to S3 `gruesome-frontend`
- CloudFront distribution
- Custom domain (optional)

---

## Implementation Checklist

### Backend (Phase 1)

- [ ] Delete server-side interpreter code
  - [ ] Remove `lambda_wrapper.rs`
  - [ ] Remove `game_service.rs` (execution logic)
  - [ ] Remove `session_service.rs`

- [ ] Implement new services
  - [ ] `game_service.rs` - Metadata CRUD
  - [ ] `save_service.rs` - Save CRUD with S3 presigned URLs
  - [ ] Update `models.rs` - New request/response types

- [ ] Update Lambda handlers
  - [ ] Implement game endpoints
  - [ ] Implement save endpoints
  - [ ] Remove old execution endpoints

- [ ] Update CDK
  - [ ] Remove old routes
  - [ ] Add new routes
  - [ ] Update Lambda permissions (S3 presigned URL generation)

- [ ] Data migration
  - [ ] Create game metadata seeding script
  - [ ] Populate DynamoDB with game metadata
  - [ ] Clean up old session data

- [ ] Testing
  - [ ] Update test scripts
  - [ ] Test all new endpoints
  - [ ] Verify presigned URLs work
  - [ ] Test save upload/download

### Frontend (Phase 2)

- [ ] Project setup
  - [ ] Create frontend directory structure
  - [ ] Copy WASM build artifacts

- [ ] Authentication
  - [ ] Login page
  - [ ] JWT storage
  - [ ] Auth state management

- [ ] Game library
  - [ ] List games UI
  - [ ] Game metadata display
  - [ ] Game selection

- [ ] Game player
  - [ ] WASM initialization
  - [ ] Game file loading
  - [ ] Text display
  - [ ] Input handling
  - [ ] Save/Load UI

- [ ] Deployment
  - [ ] Deploy to S3
  - [ ] Configure CloudFront
  - [ ] Test production

---

## Success Criteria

**Backend Complete When:**
- ✓ All game metadata in DynamoDB
- ✓ All new API endpoints working
- ✓ Presigned URLs generating correctly
- ✓ Save upload/download tested
- ✓ All tests passing

**Frontend Complete When:**
- ✓ Users can log in
- ✓ Users can see game library
- ✓ Users can play games in browser
- ✓ Users can save/load games
- ✓ All games execute correctly in WASM

**Migration Complete When:**
- ✓ No server-side game execution code remains
- ✓ All game execution happens in browser
- ✓ Backend only handles storage/retrieval
- ✓ Lower latency than old architecture
- ✓ Lower AWS costs (no Lambda per command)

---

## Rollback Plan

If pivot fails, can restore from Git:
- Commit hash before pivot: `<to be recorded>`
- Old Lambda code preserved in git history
- DynamoDB data can coexist (different PK patterns)

---

## Cost Comparison

**Old Architecture (Server-Side):**
- Lambda: $0.20 per 1M requests × commands per session
- Compute: 512MB × 300ms per command
- DynamoDB: Write/read per command
- **Cost per 1000 commands: ~$0.50-$1.00**

**New Architecture (Client-Side):**
- Lambda: Only for save/load (1-2 per session)
- S3: Presigned URL generation (free)
- DynamoDB: Minimal reads/writes
- **Cost per 1000 commands: ~$0.01-$0.02**

**Savings: ~98% reduction in backend costs**

---

## Timeline Estimate

**Phase 1 (Backend):**
- Code restructuring: 2-3 hours
- Testing: 1 hour
- **Total: 3-4 hours**

**Phase 2 (Frontend):**
- Basic UI: 2-3 hours
- WASM integration: 1-2 hours
- Save/Load: 1-2 hours
- Testing: 1 hour
- **Total: 5-8 hours**

**Overall: 8-12 hours of focused work**

---

## Next Steps

1. Review this plan
2. Confirm approach
3. Begin Phase 1, Step 1.1 (Delete old code)
