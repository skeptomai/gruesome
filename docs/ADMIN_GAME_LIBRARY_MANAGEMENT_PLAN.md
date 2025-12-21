# Admin Game Library Management - Implementation Plan

**Priority**: 🎯 NEXT (User Priority #1)
**Date**: December 20, 2025
**Status**: Planning Phase

---

## Table of Contents

1. [Overview](#overview)
2. [Requirements](#requirements)
3. [Architecture](#architecture)
4. [Implementation Phases](#implementation-phases)
5. [Technical Specifications](#technical-specifications)
6. [Security Considerations](#security-considerations)
7. [Testing Strategy](#testing-strategy)

---

## Overview

### Goal
Create an admin interface for uploading and managing Z-Machine game files with automated metadata extraction and manual override capabilities.

### User Story
As an admin user, I want to upload new Z-Machine games to the platform, automatically extract their metadata, optionally edit the metadata, and make them available to all users in a single workflow.

### Current State
- Games manually uploaded to S3 via AWS CLI
- Metadata manually created in DynamoDB via AWS CLI
- No web interface for game management
- No automated metadata extraction
- 7 games currently in production (Zork I-III, HHGG, Planetfall, Wishbringer, Enchanter)

### Target State
- Web-based admin interface for game uploads
- Automated metadata extraction from Z-Machine headers
- Form-based metadata editing (title, description, author)
- Single-click publish workflow
- Game listing with edit/delete capabilities
- Version management for game file updates

---

## Requirements

### Functional Requirements

**FR1: Admin Authentication**
- Only authenticated admin users can access game management interface
- Admin role stored in DynamoDB user profile (`role: "admin"`)
- Non-admin users see 403 Forbidden if they attempt access

**FR2: Game File Upload**
- Accept .z3, .z4, .z5, .z8 file formats
- Maximum file size: 512 KB (covers all Infocom games)
- Client-side validation before upload
- Progress indicator during upload
- Upload directly to S3 with presigned URLs

**FR3: Metadata Extraction**
- Automatically extract from Z-Machine header:
  - Version (byte 0)
  - Release number (bytes 2-3)
  - Serial number (bytes 18-23, ASCII)
  - Checksum (bytes 28-29)
  - File length (calculate from uploaded file)
- Generate game_id from filename (lowercase, alphanumeric only)

**FR4: Metadata Editing**
- Form fields:
  - Title (required, max 100 chars)
  - Author (required, max 50 chars)
  - Description (required, max 500 chars)
  - Category (optional: "Adventure", "Mystery", "Sci-Fi", "Fantasy", "Humor")
  - Year published (optional, 1977-2025)
- Pre-populate from extracted data where available
- Validation before submission

**FR5: Game Publishing**
- Upload file to S3 bucket (gruesome-games/games/{game_id}.z{version})
- Create DynamoDB metadata entry (GAME#{game_id})
- Atomic operation (rollback on failure)
- Success/error feedback to admin

**FR6: Game Listing**
- Table view of all games with columns:
  - Title, Author, Version, Release, File Size, Upload Date
- Sort by any column
- Search/filter by title or author
- Actions: Edit, Delete, Download

**FR7: Game Editing**
- Load existing metadata into form
- Update metadata in DynamoDB
- Option to re-upload game file (new version)
- Track modification history (modified_at timestamp)

**FR8: Game Deletion**
- Soft delete (mark as archived, don't remove from S3/DynamoDB)
- Confirmation dialog before deletion
- Admin-only operation
- Option to permanently delete (remove from S3 and DynamoDB)

### Non-Functional Requirements

**NFR1: Performance**
- File upload completes in < 5 seconds for typical game (100 KB)
- Metadata extraction completes in < 1 second
- Game listing loads in < 2 seconds

**NFR2: Security**
- Admin role verification on every API call
- Presigned S3 URLs expire after 5 minutes
- File type validation (magic number check, not just extension)
- Input sanitization on all metadata fields

**NFR3: Reliability**
- Failed uploads don't leave orphaned S3 objects
- Failed metadata creation rolls back S3 upload
- Retry mechanism for transient errors

**NFR4: Usability**
- Mobile-responsive admin interface
- Clear error messages with actionable guidance
- Inline validation on form fields
- Drag-and-drop file upload

---

## Architecture

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                      Admin Frontend                         │
│  (React component in existing frontend/app.js)             │
│                                                             │
│  Components:                                                │
│  - AdminGameUpload     (file upload + metadata form)        │
│  - AdminGameList       (table of games with actions)        │
│  - AdminGameEdit       (edit metadata form)                 │
│  - AdminGameDelete     (confirmation dialog)                │
└─────────────────────────────────────────────────────────────┘
                              ↓ HTTPS
┌─────────────────────────────────────────────────────────────┐
│                    API Gateway + CloudFront                  │
│  Routes:                                                     │
│  POST   /api/admin/games/upload-url    (get presigned URL)  │
│  POST   /api/admin/games               (create metadata)    │
│  GET    /api/admin/games               (list all games)     │
│  GET    /api/admin/games/{id}          (get game metadata)  │
│  PUT    /api/admin/games/{id}          (update metadata)    │
│  DELETE /api/admin/games/{id}          (delete game)        │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│              Lambda: gruesome-admin-api (Rust)              │
│                                                             │
│  Modules:                                                   │
│  - auth.rs            (verify admin role)                   │
│  - upload.rs          (presigned URL generation)            │
│  - metadata.rs        (extract Z-Machine header data)       │
│  - games.rs           (CRUD operations)                     │
│  - validation.rs      (input validation)                    │
└─────────────────────────────────────────────────────────────┘
           ↓                                    ↓
┌──────────────────────┐           ┌────────────────────────┐
│   S3: gruesome-games │           │  DynamoDB:             │
│                      │           │  gruesome-platform     │
│   games/             │           │                        │
│   ├─ zork1.z3        │           │  GAME#{game_id}        │
│   ├─ enchanter.z3    │           │  SK: METADATA          │
│   └─ ...             │           │                        │
└──────────────────────┘           └────────────────────────┘
```

### Data Flow

**Upload Flow**:
```
1. Admin clicks "Upload Game"
2. Frontend: Select .z3 file
3. Frontend → Lambda: POST /api/admin/games/upload-url
4. Lambda: Verify admin role
5. Lambda: Generate presigned S3 URL (5 min expiry)
6. Lambda → Frontend: Return presigned URL + upload_id
7. Frontend → S3: PUT file to presigned URL (direct upload)
8. Frontend: Extract metadata from file header (WASM helper)
9. Frontend: Show metadata form (pre-populated)
10. Admin: Edit metadata fields
11. Frontend → Lambda: POST /api/admin/games (with metadata)
12. Lambda: Verify admin role
13. Lambda: Validate metadata
14. Lambda: Verify S3 file exists
15. Lambda: Create DynamoDB entry
16. Lambda → Frontend: Success response
17. Frontend: Refresh game list
```

**Edit Flow**:
```
1. Admin clicks "Edit" on game row
2. Frontend → Lambda: GET /api/admin/games/{id}
3. Lambda: Verify admin role
4. Lambda → DynamoDB: Get metadata
5. Lambda → Frontend: Return metadata
6. Frontend: Show edit form (pre-populated)
7. Admin: Edit fields
8. Frontend → Lambda: PUT /api/admin/games/{id}
9. Lambda: Verify admin role
10. Lambda: Validate metadata
11. Lambda → DynamoDB: Update metadata
12. Lambda → Frontend: Success response
13. Frontend: Refresh game list
```

### DynamoDB Schema

**Game Metadata Entry**:
```json
{
  "PK": "GAME#zork1",
  "SK": "METADATA",
  "entity_type": "GAME",
  "game_id": "zork1",
  "title": "Zork I: The Great Underground Empire",
  "author": "Infocom",
  "description": "The first in the Zork trilogy...",
  "category": "Adventure",
  "year_published": 1980,
  "s3_key": "games/zork1.z3",
  "file_size": 92160,
  "version": 3,
  "release": 88,
  "serial": "840726",
  "checksum": "80b8",
  "created_at": 1734707670,
  "created_by": "admin@example.com",
  "modified_at": 1734707670,
  "modified_by": "admin@example.com",
  "archived": false
}
```

**Admin User Entry** (enhanced):
```json
{
  "PK": "USER#f939b9ee-a0e1-70ed-e42f-c1feb82fadff",
  "SK": "PROFILE",
  "entity_type": "USER",
  "username": "admin",
  "email": "admin@example.com",
  "role": "admin",           // ← New field
  "created_at": 1734707670
}
```

### S3 Bucket Structure

```
gruesome-games/
├─ games/
│  ├─ zork1.z3              (current version)
│  ├─ zork2.z3
│  ├─ enchanter.z3
│  └─ ...
└─ archive/                  (optional: version history)
   ├─ zork1-v1.z3
   ├─ zork1-v2.z3
   └─ ...
```

---

## Implementation Phases

### Phase 1: Backend API (Lambda + DynamoDB)

**Estimated Effort**: 1-2 sessions

**Tasks**:
1. Create new Lambda function: `gruesome-admin-api` (Rust)
2. Implement admin role verification middleware
3. Implement metadata extraction from Z-Machine headers
4. Create API endpoints:
   - `POST /api/admin/games/upload-url` - Generate presigned S3 URL
   - `POST /api/admin/games` - Create game metadata
   - `GET /api/admin/games` - List all games (admin view)
   - `GET /api/admin/games/{id}` - Get game details
   - `PUT /api/admin/games/{id}` - Update game metadata
   - `DELETE /api/admin/games/{id}` - Soft delete game
5. Add validation logic for all inputs
6. Configure IAM permissions for S3 presigned URLs
7. Add API routes to API Gateway
8. Deploy to staging environment

**Deliverables**:
- `infrastructure/lambda/gruesome-admin-api/` directory
- Lambda function deployed to both environments
- API Gateway routes configured
- Unit tests for metadata extraction

**Verification**:
- Can generate presigned S3 URL via API
- Can create game metadata via API
- Can list/get/update/delete games via API
- Non-admin users get 403 Forbidden

### Phase 2: Frontend Admin Interface

**Estimated Effort**: 1-2 sessions

**Tasks**:
1. Create admin route in frontend (`/admin/games`)
2. Implement `AdminGameUpload` component:
   - File picker (drag-and-drop + click to browse)
   - Progress bar during upload
   - Metadata extraction (use WASM Z-Machine header reader)
   - Metadata form (title, author, description, category, year)
   - Validation and error display
   - Submit button (create game)
3. Implement `AdminGameList` component:
   - Table with sortable columns
   - Search/filter functionality
   - Edit/Delete action buttons
   - Pagination (if > 20 games)
4. Implement `AdminGameEdit` component:
   - Load existing metadata
   - Edit form (same fields as upload)
   - Update button
5. Implement `AdminGameDelete` component:
   - Confirmation dialog
   - Delete button
6. Add admin navigation link (visible only to admin users)
7. Style components to match existing retro terminal theme
8. Deploy to staging environment

**Deliverables**:
- Admin components in `frontend/admin/` directory
- Routing for `/admin/games`
- Styled to match existing theme
- Mobile-responsive design

**Verification**:
- Can upload .z3 file via web interface
- Metadata auto-populates from Z-Machine header
- Can edit and submit metadata
- Game appears in main library after upload
- Can list all games in admin view
- Can edit existing game metadata
- Can delete games (soft delete)

### Phase 3: Enhanced Features

**Estimated Effort**: 1 session

**Tasks**:
1. Add category filtering in main game library
2. Add year published to game cards
3. Implement version management:
   - Upload new version of existing game
   - Archive old version to S3 `archive/` prefix
   - Track version history in DynamoDB
4. Add bulk operations:
   - Select multiple games
   - Bulk delete
   - Bulk categorize
5. Add game statistics:
   - Number of times played (tracking in DynamoDB)
   - Last played timestamp
   - Popular games section
6. Deploy to production environment

**Deliverables**:
- Enhanced game library with filtering
- Version management system
- Bulk operations interface
- Game statistics tracking

**Verification**:
- Can filter games by category
- Can upload new version of existing game
- Old versions archived correctly
- Bulk operations work correctly
- Statistics track accurately

### Phase 4: Admin User Management

**Estimated Effort**: 1 session (stretch goal)

**Tasks**:
1. Create admin user management interface
2. List all users with roles
3. Promote user to admin
4. Demote admin to regular user
5. View user activity (games played, saves created)
6. Audit log for admin actions

**Deliverables**:
- User management interface
- Role assignment capabilities
- Activity viewing
- Audit logging

**Verification**:
- Can list all users
- Can promote/demote admin roles
- Can view user activity
- Audit log tracks all admin actions

---

## Technical Specifications

### Z-Machine Header Parsing

**Header Layout** (bytes 0-63):
```rust
pub struct ZMachineHeader {
    pub version: u8,           // Byte 0
    pub release: u16,          // Bytes 2-3 (big-endian)
    pub serial: String,        // Bytes 18-23 (ASCII, e.g., "840726")
    pub checksum: u16,         // Bytes 28-29 (big-endian)
    pub file_length: u32,      // Calculate from file size
}

pub fn extract_metadata(file_bytes: &[u8]) -> Result<ZMachineHeader, String> {
    if file_bytes.len() < 64 {
        return Err("File too small to be valid Z-Machine file".to_string());
    }

    let version = file_bytes[0];
    if ![3, 4, 5, 8].contains(&version) {
        return Err(format!("Unsupported Z-Machine version: {}", version));
    }

    let release = u16::from_be_bytes([file_bytes[2], file_bytes[3]]);

    let serial = String::from_utf8_lossy(&file_bytes[18..24]).to_string();

    let checksum = u16::from_be_bytes([file_bytes[28], file_bytes[29]]);

    let file_length = file_bytes.len() as u32;

    Ok(ZMachineHeader {
        version,
        release,
        serial,
        checksum,
        file_length,
    })
}
```

### API Request/Response Schemas

**POST /api/admin/games/upload-url**

Request:
```json
{
  "filename": "zork1.z3",
  "content_type": "application/octet-stream"
}
```

Response:
```json
{
  "upload_url": "https://s3.amazonaws.com/gruesome-games/games/zork1.z3?...",
  "upload_id": "uuid-v4",
  "expires_at": 1734712270
}
```

**POST /api/admin/games**

Request:
```json
{
  "game_id": "zork1",
  "title": "Zork I: The Great Underground Empire",
  "author": "Infocom",
  "description": "The first in the Zork trilogy...",
  "category": "Adventure",
  "year_published": 1980,
  "s3_key": "games/zork1.z3",
  "file_size": 92160,
  "version": 3,
  "release": 88,
  "serial": "840726",
  "checksum": "80b8"
}
```

Response:
```json
{
  "success": true,
  "game_id": "zork1",
  "message": "Game created successfully"
}
```

**GET /api/admin/games**

Response:
```json
{
  "games": [
    {
      "game_id": "zork1",
      "title": "Zork I: The Great Underground Empire",
      "author": "Infocom",
      "version": 3,
      "release": 88,
      "file_size": 92160,
      "created_at": 1734707670,
      "modified_at": 1734707670
    },
    // ... more games
  ],
  "total": 7
}
```

### File Validation

**Magic Number Check** (WASM/Rust):
```rust
pub fn validate_zmachine_file(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 64 {
        return Err("File too small (minimum 64 bytes required)".to_string());
    }

    // Check version byte (must be 3, 4, 5, or 8)
    let version = bytes[0];
    if ![3, 4, 5, 8].contains(&version) {
        return Err(format!("Invalid Z-Machine version: {} (expected 3, 4, 5, or 8)", version));
    }

    // Check high memory mark is reasonable (bytes 4-5)
    let high_mem = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    if high_mem > bytes.len() {
        return Err(format!("Invalid high memory mark: 0x{:04x} exceeds file size", high_mem));
    }

    // Check initial PC is reasonable (bytes 6-7)
    let initial_pc = u16::from_be_bytes([bytes[6], bytes[7]]) as usize;
    if initial_pc >= bytes.len() {
        return Err(format!("Invalid initial PC: 0x{:04x} exceeds file size", initial_pc));
    }

    Ok(())
}
```

---

## Security Considerations

### Authentication & Authorization

**Admin Role Verification**:
- Every admin API endpoint checks user role from DynamoDB
- Role stored in USER#{user_id}/PROFILE
- JWT token contains user_id, Lambda looks up role
- 403 Forbidden returned if not admin

**Implementation**:
```rust
pub async fn verify_admin(user_id: &str, dynamodb: &Client) -> Result<(), ApiError> {
    let result = dynamodb
        .get_item()
        .table_name("gruesome-platform")
        .key("PK", AttributeValue::S(format!("USER#{}", user_id)))
        .key("SK", AttributeValue::S("PROFILE".to_string()))
        .send()
        .await?;

    let item = result.item().ok_or(ApiError::NotFound)?;

    let role = item
        .get("role")
        .and_then(|v| v.as_s().ok())
        .unwrap_or("user");

    if role != "admin" {
        return Err(ApiError::Forbidden("Admin access required".to_string()));
    }

    Ok(())
}
```

### Input Validation

**File Upload**:
- Maximum file size: 512 KB (enforced in presigned URL policy)
- File type validation via magic number check
- Filename sanitization (alphanumeric + dash/underscore only)

**Metadata Fields**:
- Title: 1-100 characters, no HTML tags
- Author: 1-50 characters, no HTML tags
- Description: 1-500 characters, no HTML tags
- Category: Enum validation (Adventure, Mystery, etc.)
- Year: 1977-2025 range

**Implementation**:
```rust
pub fn validate_game_metadata(metadata: &GameMetadata) -> Result<(), String> {
    if metadata.title.is_empty() || metadata.title.len() > 100 {
        return Err("Title must be 1-100 characters".to_string());
    }

    if metadata.title.contains('<') || metadata.title.contains('>') {
        return Err("Title cannot contain HTML tags".to_string());
    }

    // Similar validation for other fields...

    Ok(())
}
```

### S3 Security

**Presigned URL Policy**:
```rust
let presigned_request = s3_client
    .put_object()
    .bucket("gruesome-games")
    .key(format!("games/{}", sanitized_filename))
    .content_type("application/octet-stream")
    .content_length(max_file_size)  // 512 KB
    .presigned(
        PresigningConfig::builder()
            .expires_in(Duration::from_secs(300))  // 5 minutes
            .build()?,
    )
    .await?;
```

**Bucket Policy**:
- Only Lambda execution role can create presigned URLs
- Public read access for game files (served via CloudFront)
- No public write access

### Audit Logging

**Log all admin actions**:
- Who performed the action (user_id, username)
- What action (create, update, delete)
- When (timestamp)
- Which resource (game_id)
- Result (success/failure)

**Implementation**:
```rust
pub async fn log_admin_action(
    action: &str,
    user_id: &str,
    resource_id: &str,
    success: bool,
) {
    log::info!(
        "ADMIN_ACTION: user={} action={} resource={} success={}",
        user_id,
        action,
        resource_id,
        success
    );

    // Optional: Write to DynamoDB audit log table
    // AUDIT#{timestamp}#SK#{user_id}
}
```

---

## Testing Strategy

### Unit Tests

**Lambda Functions**:
- Test metadata extraction with sample Z-Machine files
- Test validation logic with valid/invalid inputs
- Test admin role verification with different user roles
- Test presigned URL generation

**Frontend Components**:
- Test file upload component with mock files
- Test metadata form validation
- Test game list filtering and sorting
- Test edit/delete confirmation flows

### Integration Tests

**API Endpoints**:
- Upload complete workflow (presigned URL → S3 upload → metadata creation)
- Edit workflow (get metadata → update → verify)
- Delete workflow (soft delete → verify archived flag)
- Access control (non-admin gets 403)

**End-to-End**:
- Upload new game via web interface
- Verify game appears in library
- Edit game metadata
- Verify changes reflected in library
- Delete game
- Verify game no longer appears in library

### Manual Testing Checklist

**Upload Flow**:
- [ ] Upload .z3 file via drag-and-drop
- [ ] Upload .z3 file via file picker
- [ ] Metadata auto-extracts correctly
- [ ] Can edit all metadata fields
- [ ] Validation errors display correctly
- [ ] Game appears in library after submit
- [ ] Upload fails gracefully on error

**Edit Flow**:
- [ ] Click edit on game row
- [ ] Metadata pre-populates correctly
- [ ] Can update all fields
- [ ] Changes save correctly
- [ ] Changes reflect in library

**Delete Flow**:
- [ ] Click delete on game row
- [ ] Confirmation dialog appears
- [ ] Game removed from library after confirm
- [ ] Game file still in S3 (soft delete)
- [ ] Cancel dialog keeps game in library

**Access Control**:
- [ ] Admin user sees admin menu
- [ ] Regular user does not see admin menu
- [ ] Regular user gets 403 on admin API calls
- [ ] Admin can perform all operations

**Error Handling**:
- [ ] Invalid file type shows error
- [ ] File too large shows error
- [ ] Network error during upload shows retry option
- [ ] Invalid metadata shows specific error messages

---

## Success Criteria

### Phase 1 Complete When:
- [ ] Admin Lambda API deployed to staging
- [ ] All 6 API endpoints functional
- [ ] Admin role verification working
- [ ] Presigned S3 URLs generated correctly
- [ ] Metadata extraction from Z-Machine headers working
- [ ] Unit tests passing

### Phase 2 Complete When:
- [ ] Admin interface deployed to staging
- [ ] Can upload game file via web interface
- [ ] Metadata auto-extracts and pre-populates form
- [ ] Can create game with custom metadata
- [ ] Game appears in main library
- [ ] Can list all games in admin view
- [ ] Can edit game metadata
- [ ] Can delete games (soft delete)

### Phase 3 Complete When:
- [ ] Category filtering works in main library
- [ ] Version management system functional
- [ ] Bulk operations available
- [ ] Game statistics tracking working
- [ ] Deployed to production

### Overall Success:
- [ ] Admin can upload games without using AWS CLI
- [ ] Game metadata managed entirely via web interface
- [ ] New games available to users within minutes of upload
- [ ] System is reliable and error-resistant
- [ ] Mobile-friendly admin interface

---

## Next Steps

1. **Review this plan** with user to confirm approach
2. **Create admin role** in DynamoDB for testing user
3. **Begin Phase 1**: Implement backend Lambda API
4. **Test API endpoints** in staging environment
5. **Begin Phase 2**: Implement frontend admin interface
6. **Integration testing** with complete upload workflow
7. **Deploy to production** after staging verification

---

## Open Questions

1. Should we support uploading multiple files at once (batch upload)?
2. Should deleted games be permanently removed after X days, or keep archived forever?
3. Should regular users be able to request games to be added (submission queue)?
4. Should we track download/play statistics per game?
5. Should we support game cover art/screenshots in metadata?

---

**Document Status**: Ready for Implementation
**Next Action**: Begin Phase 1 - Backend API Development
