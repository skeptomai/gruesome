# Multi-User WASM Z-Machine Platform Architecture

## Core Requirements Analysis

**What We Need:**
1. User identity (login/registration)
2. Per-user game save storage
3. Multiple concurrent games per user
4. Save game persistence across sessions
5. (Potentially) Social features (leaderboards, achievements, sharing)

---

## Architecture Options

### Option 1: Serverless + Static (Simplest)

**Stack:**
- **Frontend**: Current WASM setup (unchanged)
- **Auth**: Firebase Authentication or AWS Cognito
- **Storage**: Firebase Firestore or AWS S3 with signed URLs
- **Hosting**: Current GitHub Pages + serverless backend

**Flow:**
1. User logs in via Firebase/Cognito (OAuth/social or email/password)
2. Frontend gets auth token
3. Save game → upload to Firebase Storage/S3 with user ID prefix: `saves/{userId}/{gameId}/{timestamp}.qzl`
4. Restore game → fetch from storage using auth token

**Pros:**
- Minimal backend code
- GitHub Pages stays for static assets
- Firebase/Cognito handles all auth complexity
- Pay-per-use pricing (free tier generous)

**Cons:**
- Vendor lock-in
- Limited control over auth flow
- CORS complexity with S3

---

### Option 2: Lightweight Backend (Recommended)

**Stack:**
- **Frontend**: Current WASM + login UI
- **Backend**: Rust (Axum/Actix) or Go (Echo/Gin) API
- **Auth**:
  - **Self-hosted**: `jsonwebtoken` crate with bcrypt passwords
  - **OAuth**: GitHub/Google via OAuth2 (passport.js pattern)
- **Database**: PostgreSQL (user accounts, game metadata)
- **Storage**: S3-compatible (AWS S3, Backblaze B2, or MinIO)
- **Hosting**:
  - Frontend: GitHub Pages, Cloudflare Pages, Netlify, or Vercel
  - Backend: Fly.io, Railway, or AWS Lambda + API Gateway

**API Endpoints:**
```
POST   /api/auth/register          - Create account
POST   /api/auth/login             - Get JWT token
POST   /api/auth/oauth/github      - OAuth login
GET    /api/games                  - List user's saved games
POST   /api/games/{gameId}/save    - Upload save file
GET    /api/games/{gameId}/saves   - List saves for a game
GET    /api/games/{gameId}/saves/{saveId} - Download save
DELETE /api/games/{gameId}/saves/{saveId} - Delete save
```

**Database Schema (DynamoDB Single-Table Design):**
```
Table: gruesome-platform

Partition Key (PK): String
Sort Key (SK): String

Item Types:

1. User Profile:
   PK: USER#{user_id}
   SK: PROFILE
   Attributes: {
       username: String,
       email: String,
       oauth_provider: String,  // 'github', 'google', 'cognito'
       oauth_id: String,
       created_at: Number (Unix timestamp)
   }

2. Game Metadata:
   PK: USER#{user_id}
   SK: GAME#{game_checksum}
   Attributes: {
       game_name: String,       // "Zork I", "Trinity"
       game_checksum: String,   // "r88-s840726"
       last_played: Number,
       total_playtime: Number,
       save_count: Number
   }

3. Save File Metadata:
   PK: USER#{user_id}
   SK: SAVE#{game_checksum}#{save_id}
   Attributes: {
       save_name: String,       // "West of House" or "Auto Save"
       s3_key: String,          // saves/{user_id}/{game}/{save_id}.qzl
       created_at: Number,
       file_size: Number,
       game_checksum: String    // For querying
   }

Global Secondary Index (GSI1):
   GSI1PK: email (for login by email)
   GSI1SK: USER#{user_id}

Query Patterns Supported:
- Get user profile: PK=USER#{id}, SK=PROFILE
- Get all games for user: PK=USER#{id}, SK begins_with("GAME#")
- Get all saves for user+game: PK=USER#{id}, SK begins_with("SAVE#{checksum}#")
- Login by email: GSI1PK=email
```

**Why Single-Table Design:**
- **Performance**: Single query retrieves all related data
- **Cost**: Fewer read units, no joins
- **Scalability**: DynamoDB optimizes partition distribution
- **Access Patterns**: All queries follow user-centric access

**Rust Backend Example (Axum with DynamoDB):**
```rust
use aws_sdk_dynamodb::Client as DynamoClient;
use aws_sdk_s3::Client as S3Client;
use axum::{Router, routing::{get, post}, extract::{Path, State}, Json};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct AppState {
    dynamo: DynamoClient,
    s3: S3Client,
}

#[tokio::main]
async fn main() {
    let config = aws_config::load_from_env().await;
    let state = AppState {
        dynamo: DynamoClient::new(&config),
        s3: S3Client::new(&config),
    };

    let app = Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/games/:game_id/save", post(save_game))
        .route("/api/games/:game_id/saves/:save_id", get(load_save))
        .route("/api/games", get(list_games))
        .with_state(state)
        .layer(CorsLayer::permissive());

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn save_game(
    State(state): State<AppState>,
    Path((game_checksum, save_id)): Path<(String, String)>,
    auth: AuthUser,  // Extracted from Cognito JWT
    body: Bytes,
) -> Result<Json<SaveResponse>> {
    // Save file to S3
    let s3_key = format!("saves/{}/{}/{}.qzl", auth.user_id, game_checksum, save_id);

    state.s3.put_object()
        .bucket("gruesome-saves")
        .key(&s3_key)
        .body(body.into())
        .send()
        .await?;

    // Save metadata to DynamoDB
    state.dynamo.put_item()
        .table_name("gruesome-platform")
        .item("PK", AttributeValue::S(format!("USER#{}", auth.user_id)))
        .item("SK", AttributeValue::S(format!("SAVE#{}#{}", game_checksum, save_id)))
        .item("save_name", AttributeValue::S("Auto Save".to_string()))
        .item("s3_key", AttributeValue::S(s3_key.clone()))
        .item("created_at", AttributeValue::N(chrono::Utc::now().timestamp().to_string()))
        .item("file_size", AttributeValue::N(body.len().to_string()))
        .item("game_checksum", AttributeValue::S(game_checksum.clone()))
        .send()
        .await?;

    Ok(Json(SaveResponse { save_id, s3_key }))
}

async fn list_games(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<GameInfo>>> {
    // Query all games for user
    let result = state.dynamo.query()
        .table_name("gruesome-platform")
        .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
        .expression_attribute_values(":pk", AttributeValue::S(format!("USER#{}", auth.user_id)))
        .expression_attribute_values(":sk_prefix", AttributeValue::S("GAME#".to_string()))
        .send()
        .await?;

    let games = result.items.unwrap_or_default()
        .into_iter()
        .map(|item| GameInfo::from_dynamodb_item(item))
        .collect();

    Ok(Json(games))
}
```

**Pros:**
- Full control over auth and data
- Can add features (achievements, multiplayer observations)
- Portable (not locked to Firebase)
- Rust backend shares code with WASM

**Cons:**
- More infrastructure to manage
- Need to handle database backups
- More complex deployment

---

### Option 3: Full Platform (Future-Proof)

**Stack:**
- Above + WebSocket support for real-time features
- Redis for session management
- CDN for WASM delivery
- Monitoring (Prometheus/Grafana)

**Additional Features:**
- Live spectating (watch others play)
- Shared save files (community puzzles)
- Achievements/leaderboards
- Social graph (friends, follows)

---

## Authentication Deep Dive

### OAuth vs Email/Password

**OAuth (Recommended Start):**
- **Providers**: GitHub, Google, Discord
- **Library**: `oauth2` crate (Rust) or NextAuth.js (if using Node)
- **Flow**:
  1. User clicks "Login with GitHub"
  2. Redirect to GitHub OAuth
  3. GitHub redirects back with code
  4. Exchange code for token
  5. Fetch user profile
  6. Create user if new, else login
  7. Issue JWT for your API

**Why OAuth First:**
- No password storage liability
- Users prefer social login
- Fast implementation
- Free (no email service needed)

**Email/Password (Add Later):**
- Use `bcrypt` or `argon2` for hashing
- Require email verification
- Password reset flow needed
- More code, more attack surface

### JWT Token Strategy

```rust
struct Claims {
    sub: String,      // user_id
    exp: i64,         // expiration (24h)
    iat: i64,         // issued at
    username: String,
}
```

Store in `localStorage` on client, send in `Authorization: Bearer <token>` header.

**Security:**
- HTTPS only (free via Let's Encrypt)
- HttpOnly cookies if you want CSRF protection
- Token refresh endpoint for long sessions

---

## Storage Strategy

### S3 vs Database Blobs

**S3 (or compatible):**
- **Pros**: Designed for file storage, cheap, scalable, CDN integration
- **Cons**: Extra service, eventual consistency
- **Cost**: AWS S3: $0.023/GB/month, Backblaze B2: $0.005/GB/month

**PostgreSQL Blobs:**
- **Pros**: Same database, transactional, simpler
- **Cons**: Not designed for files, backup size bloat
- **Limit**: Save files are ~50-200KB each, manageable

**Recommendation**: S3 for production, Postgres for MVP/testing.

### Save File Organization

```
s3://gruesome-saves/
  users/
    {user_id}/
      zork1-r88-s840726/     # game checksum
        autosave.qzl          # Latest autosave
        manual-001.qzl        # User-named saves
        manual-002.qzl
        checkpoint-west-of-house.qzl
      trinity-r11-s860509/
        autosave.qzl
```

**Metadata in DB, Blobs in S3:**
- Fast queries (list saves)
- Efficient storage
- Easy CDN integration

---

## Database Architecture Deep Dive

### Why DynamoDB Over RDS PostgreSQL

**Selected Architecture**: DynamoDB for all relational data, S3 for save file blobs

#### RDS Usage in Original Architecture

The RDS database was storing three types of data:

1. **User Accounts** (`users` table)
   - Username, email, OAuth provider info
   - ~1KB per user
   - Total: ~100KB for 100 users, ~1MB for 1000 users

2. **Game Metadata** (`games` table)
   - Which games each user has played
   - Game name, checksum, last played timestamp
   - ~500 bytes per game per user
   - Total: ~50KB for 100 users × 1 game, ~500KB for 1000 users × 1 game

3. **Save File Metadata** (`saves` table)
   - Save file names, timestamps, S3 keys
   - Actual save data lives in S3 (not in database)
   - ~200 bytes per save file
   - Total: ~200KB for 1000 saves, ~2MB for 10k saves

**Total data volume**: ~1-5MB for minimal usage, ~10-50MB for medium usage

**RDS cost**: $16.79/month minimum (db.t3.micro with 20GB storage)

**Problem**: Paying for 20GB minimum when only using <50MB - massive over-provisioning!

#### DynamoDB Architecture

**Cost Model**: Pay only for what you use
- Storage: $0.25/GB/month (vs $0.115/GB for RDS, but no minimum)
- Reads: $0.25 per million reads (on-demand pricing)
- Writes: $1.25 per million writes (on-demand pricing)

**Free Tier** (permanent):
- 25GB storage
- 25 read capacity units (200M requests/month)
- 25 write capacity units (25M writes/month)

**Actual costs for our usage**:
- Minimal (100 users): ~1MB data, ~10k requests/month = **$0** (free tier)
- Medium (1k users): ~10MB data, ~100k requests/month = **$0** (free tier)
- Heavy (10k users): ~100MB data, ~1M requests/month = **$0.25** (storage) + **$0** (requests) = **$0.25/month**

**Savings**: $16.79/month → $0/month for minimal/medium usage (100% reduction!)

#### DynamoDB vs Other Alternatives

**Option 1: Aurora Serverless v2**
- **Cost**: ~$0.50/month when idle (0.5 ACU minimum)
- **Pros**: PostgreSQL compatible, scales automatically
- **Cons**: Still costs money even when idle, cold start delays
- **Verdict**: Better than RDS but not as good as DynamoDB for this use case

**Option 2: SQLite in Lambda**
- **Cost**: $0 for database (just Lambda execution)
- **Pros**: Zero database cost, simple
- **Cons**: Single-writer limitation, must download/upload on each request
- **Verdict**: Too slow and limited for concurrent users

**Option 3: Cognito User Pools + S3 Metadata Only**
- **Cost**: $0 (no database at all)
- **Pros**: Extremely simple, no database to manage
- **Cons**: Can't query "all games for user" easily, must list S3 objects
- **Verdict**: Too limited for game library management

**Winner: DynamoDB**
- Effectively free for our usage volume
- Fast (single-digit millisecond latency)
- Scales automatically
- No minimum cost or provisioning
- AWS-native (integrates with Cognito, Lambda, S3)

### DynamoDB Single-Table Design Pattern

**Why Single Table Instead of Multiple Tables?**

Traditional relational thinking: One table per entity (users, games, saves)

DynamoDB best practice: **One table for entire application**

**Reasons**:

1. **Performance**: DynamoDB is optimized for single-partition queries
   - Multi-table queries require multiple round trips
   - Single table allows fetching all related data in one query

2. **Cost**: Each query costs money
   - Single query = single read unit
   - Three queries = three read units = 3x cost

3. **Atomic Operations**: All related data in same partition
   - Can use transactions across related items
   - Conditional writes work better

4. **Access Patterns**: Our queries are user-centric
   - "Get all games for user X"
   - "Get all saves for user X's game Y"
   - All queries start with user ID → perfect for partition key

**How It Works**:

```
Partition Key (PK) = USER#{user_id}
Sort Key (SK) = Type-specific identifier

Examples:
PK: USER#abc123    SK: PROFILE             → User profile data
PK: USER#abc123    SK: GAME#zork1-r88      → Zork I game metadata
PK: USER#abc123    SK: SAVE#zork1-r88#001  → Save file #1 for Zork I
```

**Query Examples**:

```rust
// Get user profile
query(PK = "USER#abc123", SK = "PROFILE")

// Get all games for user
query(PK = "USER#abc123", SK begins_with "GAME#")

// Get all saves for specific game
query(PK = "USER#abc123", SK begins_with "SAVE#zork1-r88#")

// Get single save
get_item(PK = "USER#abc123", SK = "SAVE#zork1-r88#001")
```

**All queries retrieve data in a single round trip!**

### DynamoDB vs PostgreSQL Trade-offs

**What We Lose with DynamoDB**:

1. **No SQL Queries**
   - Can't do complex JOINs or ad-hoc queries
   - Must design access patterns upfront
   - **Impact**: Minimal - our queries are simple and user-centric

2. **No Schema Enforcement**
   - DynamoDB is schema-less (NoSQL)
   - Application must validate data
   - **Impact**: Low - Rust type system handles this

3. **Limited Querying**
   - Can only query by partition key + sort key
   - Need GSI (Global Secondary Index) for other access patterns
   - **Impact**: Low - we only need email lookup (already have GSI)

4. **No Relational Integrity**
   - No foreign keys or cascade deletes
   - Application must handle orphaned records
   - **Impact**: Low - simple data model with clear ownership

**What We Gain with DynamoDB**:

1. **Zero Base Cost**
   - Free tier covers almost all usage
   - Only pay when exceeding 25GB or 200M requests
   - **Savings**: $16.79/month minimum

2. **Infinite Scalability**
   - No provisioning required
   - Scales to millions of requests automatically
   - No instance size limits

3. **Predictable Performance**
   - Single-digit millisecond latency guaranteed
   - No connection pool limits
   - No slow queries

4. **No Maintenance**
   - No backups to configure (automatic)
   - No patches to apply
   - No failover to test

5. **AWS Integration**
   - Native Lambda integration (no connection overhead)
   - DynamoDB Streams for real-time processing
   - IAM permissions instead of database users

**Verdict**: For this application, DynamoDB's benefits vastly outweigh the limitations.

### Migration Path from RDS (if needed)

If you ever need PostgreSQL features:

1. **Start with DynamoDB** (recommended)
2. If complex queries needed, add read replica in Aurora Serverless
3. Use DynamoDB as source of truth, Aurora for analytics
4. Keep costs low by only running Aurora when needed

**Most likely**: Never need PostgreSQL for this application.

---

## Frontend Changes

**Current:**
```javascript
// Save
const blob = new Blob([result.save_data]);
downloadFile(blob, "save.qzl");  // Browser download

// Restore
const file = await filePicker();
const data = await file.arrayBuffer();
```

**New:**
```javascript
// Save
const saveData = result.save_data;
await fetch('/api/games/zork1/save', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${userToken}`,
    'Content-Type': 'application/octet-stream'
  },
  body: saveData
});

// Restore
const response = await fetch(`/api/games/zork1/saves/${saveId}`, {
  headers: { 'Authorization': `Bearer ${userToken}` }
});
const saveData = await response.arrayBuffer();
interpreter.provide_restore_data(new Uint8Array(saveData));
```

**UI Additions:**
- Login/register page
- Save game browser (list with timestamps)
- Save naming dialog
- "Cloud save" vs "local download" toggle

---

## Cost Analysis: AWS DynamoDB Architecture (SELECTED)

### AWS All-In Stack with DynamoDB

**Services:**
- **Frontend**: S3 + CloudFront
- **Backend**: Lambda + API Gateway
- **Auth**: Cognito
- **Database**: DynamoDB (serverless, pay-per-request)
- **Storage**: S3 (for save files)

#### Monthly Costs Breakdown

**Minimal Usage (100 users, 1000 saves/month):**
- **S3 (frontend)**: $0.50 (1GB WASM bundle + HTML)
- **CloudFront**: $0 (free tier: 1TB transfer/month, 10M requests)
- **Lambda**: $0 (free tier: 1M requests/month)
- **API Gateway**: $0 (free tier: 1M requests/month)
- **Cognito**: $0 (free tier: 50k MAU)
- **DynamoDB**: $0 (free tier: 25GB storage, 200M requests/month)
- **S3 (saves)**: $0.46 (20GB at $0.023/GB - 1000 saves × 20KB avg)

**Total**: **~$1/month** (effectively free within AWS free tier)

**Medium Usage (1000 users, 10k saves/month):**
- **S3 (frontend)**: $0.50
- **CloudFront**: $0 (still within 1TB free tier)
- **Lambda**: $0 (still within free tier)
- **API Gateway**: $0 (still within free tier for this volume)
- **Cognito**: $0 (still under 50k MAU)
- **DynamoDB**: $0 (still within free tier - ~1GB data, ~1M requests/month)
- **S3 (saves)**: $4.60 (200GB - 10k saves × 20KB avg)

**Total**: **~$5/month**

**Heavy Usage (10k users, 100k saves/month):**
- **S3 (frontend)**: $0.50
- **CloudFront**: $8.50 (100GB transfer beyond free tier at $0.085/GB)
- **Lambda**: $2 (minimal beyond free tier)
- **API Gateway**: $3.50 (1M requests at $3.50/million)
- **Cognito**: $0 (still under 50k MAU)
- **DynamoDB**: $2.50 (10GB at $0.25/GB, 10M writes at $0.25/million beyond free tier)
- **S3 (saves)**: $46 (2TB - 100k saves × 20KB avg)

**Total**: **~$63/month**

### Previous RDS-Based Architecture (For Comparison)

**Minimal Usage**: ~$21/month (RDS was $16.79 of that)
**Medium Usage**: ~$66/month
**Heavy Usage**: ~$574/month

### Cost Savings with DynamoDB

| Usage Level | RDS Architecture | DynamoDB Architecture | Savings |
|-------------|------------------|----------------------|---------|
| **Minimal (100 users)** | $21/month | $1/month | **$20/month (95%)** |
| **Medium (1k users)** | $66/month | $5/month | **$61/month (92%)** |
| **Heavy (10k users)** | $574/month | $63/month | **$511/month (89%)** |

### Multi-Vendor Stack

**Services:**
- **Frontend**: GitHub Pages (free)
- **Backend**: Fly.io
- **Auth**: Self-hosted OAuth
- **Database**: Fly.io Postgres
- **Storage**: Backblaze B2

#### Monthly Costs Breakdown

**Minimal Usage (100 users, 1000 saves/month):**
- **GitHub Pages**: $0 (free)
- **Fly.io (backend)**: $0 (free tier: 3 shared-cpu-1x VMs)
- **Fly.io Postgres**: $0 (free tier: 3GB)
- **Backblaze B2**: $0 (free tier: 10GB)

**Total**: **$0/month**

**Medium Usage (1000 users, 10k saves/month):**
- **GitHub Pages**: $0 (still within limits)
- **Fly.io (backend)**: $5.69 (1 dedicated-cpu-1x)
- **Fly.io Postgres**: $0 (still within 3GB)
- **Backblaze B2**: $1 (200GB at $0.005/GB)

**Total**: **~$7/month**

**Heavy Usage (10k users, 100k saves/month):**
- **Cloudflare Pages**: $0 (upgraded from GitHub for better limits, still free)
- **Fly.io (backend)**: $22.76 (4 dedicated-cpu-1x for redundancy)
- **Fly.io Postgres**: $29 (production 40GB)
- **Backblaze B2**: $10 (2TB at $0.005/GB)

**Total**: **~$62/month**

### Cost Comparison Summary

| Usage Level | AWS All-In | Multi-Vendor | AWS Premium |
|-------------|------------|--------------|-------------|
| **Minimal (100 users)** | $21/month | $0/month | $21/month more |
| **Medium (1k users)** | $66/month | $7/month | $59/month more |
| **Heavy (10k users)** | $574/month | $62/month | $512/month more |

### What You're Paying For with AWS

**The $21-60/month premium buys you:**

1. **Zero Configuration Integration**
   - Cognito validates JWT tokens in API Gateway automatically
   - S3/CloudFront integration is native (no CORS)
   - IAM roles connect everything securely

2. **Managed Services**
   - RDS handles backups, patches, failover
   - Cognito handles OAuth flows, token refresh, MFA
   - No manual OAuth implementation needed

3. **Enterprise Features**
   - CloudWatch unified logging/monitoring
   - X-Ray distributed tracing
   - Cognito advanced security (compromised credentials detection)
   - S3 versioning, lifecycle policies

4. **Scalability Ceiling**
   - Lambda scales to millions of requests
   - RDS can upgrade to massive instances
   - CloudFront handles global traffic

5. **Single Vendor Support**
   - One company to call
   - One dashboard
   - Unified billing

### Multi-Vendor Premium Features

**What you lose by NOT going AWS:**
- No managed OAuth (write it yourself)
- Manual monitoring setup (multiple dashboards)
- CORS configuration between services
- Multiple vendors to manage
- Fly.io has less mature enterprise features

**What you gain:**
- Significant cost savings ($500+/month at scale)
- Portability (not locked to AWS)
- Simpler mental model for small scale
- Free tier actually free

### Recommendation: AWS All-In with DynamoDB (SELECTED)

**Architecture Decision:**

**AWS Services:**
- Frontend: S3 + CloudFront
- Backend: Lambda + API Gateway
- Auth: Cognito User Pools
- Database: **DynamoDB** (eliminates RDS cost!)
- Storage: S3 (for .qzl save files)

**Cost Benefits:**
- Minimal usage: **~$1/month** (vs $21/month with RDS)
- Medium usage: **~$5/month** (vs $66/month with RDS)
- Heavy usage: **~$63/month** (vs $574/month with RDS)
- **89-95% cost reduction** by using DynamoDB instead of RDS

**Single-Vendor Benefits:**
- All AWS services integrate seamlessly
- Cognito validates JWT in API Gateway automatically
- S3/CloudFront native integration (no CORS)
- IAM roles connect everything securely
- CloudFormation for infrastructure-as-code
- Single dashboard and billing

**AWS Free Tier:**
- Lambda: 1M requests/month (permanent)
- API Gateway: 1M requests/month (first 12 months)
- DynamoDB: 25GB + 200M requests/month (permanent)
- S3: 5GB storage (first 12 months)
- CloudFront: 1TB transfer (first 12 months)
- **Effective cost**: ~$0-1/month for first year, then ~$1-5/month ongoing

**Why DynamoDB is Perfect Here:**
- Zero base cost (vs $16.79/month minimum for RDS)
- Scales automatically (no provisioning)
- Single-digit millisecond latency
- Permanent free tier (not just 12 months)
- AWS-native (no connection overhead in Lambda)

**Long-term:**
- AWS with DynamoDB costs LESS than multi-vendor at small/medium scale
- Scales professionally when needed
- Single vendor = simpler operations
- Best of both worlds: AWS integration + low cost

---

## Static Hosting Comparison

### GitHub Pages (Current)
- **Cost**: Free
- **Deploy**: GitHub Actions (already configured)
- **CDN**: GitHub's CDN (decent)
- **Custom Domain**: Free HTTPS
- **Limits**: 100GB bandwidth/month, 1GB storage
- **Best For**: Current usage, MVP

### CloudFront + S3 (AWS Native)
- **Cost**: ~$1-5/month (free tier: 1TB transfer)
- **Deploy**: AWS CLI or GitHub Actions
- **CDN**: CloudFront (AWS's global CDN)
- **Custom Domain**: Route53 required (~$0.50/month)
- **Limits**: None (pay as you grow)
- **Best For**: All-AWS architecture

### Cloudflare Pages
- **Cost**: Free (or $20/month for teams)
- **Deploy**: Git integration (like GitHub Pages)
- **CDN**: Cloudflare's CDN (excellent, larger network than CloudFront)
- **Custom Domain**: Free HTTPS, easier DNS
- **Limits**: 500 builds/month (free), unlimited bandwidth
- **Best For**: Better performance than GitHub Pages, still free

**Why Cloudflare Pages over GitHub Pages:**
- Faster CDN (more POPs worldwide)
- Better caching
- More generous limits
- Free unlimited bandwidth

**Why CloudFront + S3 for AWS:**
- Native integration (no CORS issues)
- Same IAM policies for frontend and backend
- CloudFront integrates with Lambda@Edge
- Single vendor simplifies architecture

---

## AWS vs Multi-Vendor Decision Matrix

| Consideration | AWS Native | Multi-Vendor |
|--------------|------------|--------------|
| **Cost** | Higher ($15-50/month) | Lower ($0-20/month) |
| **Integration** | Seamless (Cognito, IAM, S3, Lambda) | Manual OAuth, separate systems |
| **Complexity** | Medium (AWS learning curve) | Low-Medium (simpler services) |
| **Vendor Lock-in** | High | Low |
| **Scalability** | Excellent | Good |
| **Monitoring** | CloudWatch (unified) | Multiple dashboards |
| **Auth** | Cognito (turnkey OAuth) | Self-hosted OAuth |
| **Best For** | Production, enterprise | MVP, indie projects |

**Recommendation:**
- **Start**: Multi-vendor (GitHub Pages + Fly.io + B2) for MVP
- **Scale**: Migrate to AWS when hitting GitHub Pages limits or needing enterprise features

---

## Implementation Phases

### Phase 1: Auth Only
1. Add login UI to existing WASM site
2. Implement OAuth GitHub login
3. Issue JWT tokens
4. Protected routes (show username)

### Phase 2: Cloud Saves
1. S3/B2 integration
2. Save upload/download endpoints
3. UI for save management
4. Migrate existing localStorage saves

### Phase 3: Game Management
1. Multiple games per user
2. Game library UI
3. Recently played tracking
4. Save organization

### Phase 4: Social (Optional)
1. Public profiles
2. Achievement system
3. Leaderboards
4. Share saves (puzzle challenges)

---

## Key Decisions

### 1. Auth Provider
- **Start**: GitHub OAuth (technical audience)
- **Add Later**: Google, Discord, email/password

### 2. Backend Language
- **Rust**: Shares code with WASM, type-safe, fast
- **Go**: Simpler deployment, easier for teams
- **TypeScript**: If frontend team maintains

### 3. Storage
- **S3 compatible** (B2 for cost, S3 for AWS ecosystem)

### 4. Database
- **DynamoDB** (serverless, pay-per-use, zero base cost) - **SELECTED**

### 5. Hosting Strategy
- **MVP**: GitHub Pages + Fly.io (free tier, simple)
- **AWS Production**: CloudFront + S3 + Lambda + Cognito + RDS (integrated)
- **Multi-Vendor Production**: Cloudflare Pages + Railway + B2 (cost-optimized)

### 6. CDN Choice
- **AWS Stack**: CloudFront (native S3 integration, Lambda@Edge)
- **Multi-Vendor**: Cloudflare (free, faster than GitHub CDN)
- **GitHub Pages**: Built-in CDN (adequate for MVP)

---

## Next Steps

1. **Decide on auth**: OAuth only, or email/password too?
2. **Choose backend framework**: Axum (Rust) or alternative?
3. **Pick storage**: Backblaze B2 (cheap) or AWS S3 (ecosystem)?
4. **Select hosting path**: All-AWS vs Multi-vendor?
5. **Design user experience**: How should save management feel?

---

## Questions to Resolve

### Static Hosting
- **GitHub Pages**: Already working, free, adequate CDN
- **CloudFront + S3**: Better if going all-AWS, native integration
- **Cloudflare Pages**: Better performance than GitHub Pages, still free

### CDN
- **CloudFront**: Use if going all-AWS (native S3 integration, Lambda@Edge)
- **Cloudflare**: Use if multi-vendor (faster, free, larger network)
- **GitHub CDN**: Use for MVP (already configured)

### Auth
- **AWS Cognito**: Use if going all-AWS (turnkey OAuth, user pools, federation)
- **Self-hosted OAuth**: Use if multi-vendor (more control, portable)

**Decision Point**: Do we optimize for cost (multi-vendor) or integration (AWS)?

**DECISION MADE**: AWS all-in with DynamoDB - gets both low cost AND integration!

---

## Final Architecture Summary (SELECTED)

### Domain Structure

**Using existing domain**: `skeptomai.com` (already on Route 53)

```
skeptomai.com
├── gruesome.skeptomai.com          → Frontend (CloudFront + S3)
└── api.gruesome.skeptomai.com      → Backend (API Gateway + Lambda)
```

**Benefits:**
- Professional branding under owned domain
- Same-origin benefits (no CORS complexity)
- Free SSL via AWS Certificate Manager
- Single vendor (all AWS infrastructure)

### Production Stack

**AWS Services (All-In):**
```
┌─────────────────────────────────────────────────────────────┐
│                         AWS Cloud                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  DNS & SSL                                                   │
│  ├─ Route 53: skeptomai.com (existing hosted zone)         │
│  └─ ACM: Free SSL cert for *.gruesome.skeptomai.com        │
│                                                              │
│  Frontend (gruesome.skeptomai.com)                          │
│  ├─ S3: Static WASM bundle + HTML/CSS/JS                   │
│  └─ CloudFront: Global CDN distribution                     │
│                                                              │
│  Backend (api.gruesome.skeptomai.com)                       │
│  ├─ Lambda: Rust functions (save/load/list)                │
│  └─ API Gateway: REST API endpoints                         │
│                                                              │
│  Auth                                                        │
│  └─ Cognito User Pools: OAuth (GitHub/Google) + JWT        │
│                                                              │
│  Data                                                        │
│  ├─ DynamoDB: User/game/save metadata (single table)       │
│  └─ S3: .qzl save file blobs                               │
│                                                              │
│  Infrastructure                                              │
│  └─ AWS CDK (TypeScript): Infrastructure as code            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Cost Projection

| Users | Monthly Cost | Breakdown |
|-------|-------------|-----------|
| **100** | **$2/month** | Route 53 ($0.50) + S3 frontend ($0.50) + S3 saves ($0.50) + CloudFront/Lambda/API GW/DynamoDB (free tier) |
| **1,000** | **$6/month** | Route 53 ($0.50) + S3 frontend ($0.50) + S3 saves ($4.50) + others (free tier) |
| **10,000** | **$64/month** | Route 53 ($0.50) + S3 frontend ($0.50) + CloudFront ($8.50) + API GW ($3.50) + Lambda ($2) + DynamoDB ($2.50) + S3 saves ($46) |

**Key Insights**:
- DynamoDB elimination of RDS reduces costs by 89-95% across all usage tiers
- All-AWS frontend adds ~$1/month (Route 53) but eliminates CORS complexity
- CloudFront replaces GitHub Pages CDN with better performance

### Database Schema (DynamoDB Single-Table)

**Table**: `gruesome-platform`

| PK (Partition Key) | SK (Sort Key) | Data Type |
|-------------------|---------------|-----------|
| USER#{id} | PROFILE | User account info |
| USER#{id} | GAME#{checksum} | Game metadata |
| USER#{id} | SAVE#{checksum}#{id} | Save file metadata |

**GSI**: `email` → USER#{id} (for login)

**S3 Save Files**: `s3://gruesome-saves/saves/{user_id}/{game_checksum}/{save_id}.qzl`

### API Endpoints

```
POST   /api/auth/register          - Create account (OAuth or email)
POST   /api/auth/login             - Login (returns JWT)
GET    /api/games                  - List user's games
POST   /api/games/{game}/save      - Upload save file
GET    /api/games/{game}/saves     - List saves for game
GET    /api/games/{game}/saves/{id} - Download save file
DELETE /api/games/{game}/saves/{id} - Delete save
```

### Why This Architecture Wins

1. **Cost**: ~$1-5/month for most usage (vs $21-66 with RDS)
2. **Simplicity**: Single vendor (AWS only)
3. **Scalability**: All serverless components scale automatically
4. **Performance**: DynamoDB <10ms, CloudFront global CDN
5. **Integration**: Native AWS service connections (no CORS, automatic JWT validation)
6. **Maintenance**: Zero servers to manage, automatic backups
7. **Development**: Rust backend shares code with WASM frontend

### Implementation Phases

**Phase 1: Infrastructure Setup (AWS CDK)**
- Initialize AWS CDK project (TypeScript)
- Define infrastructure as code:
  - ACM certificate for `*.gruesome.skeptomai.com`
  - S3 buckets (frontend + saves)
  - CloudFront distribution with custom domain
  - DynamoDB table with GSI
  - Cognito User Pool (GitHub OAuth)
  - API Gateway with custom domain
  - Lambda functions (Rust)
- Deploy via `cdk deploy` (zero manual configuration)

**Phase 2: Backend Implementation**
- Implement Lambda functions in Rust
  - Auth endpoints (login, register)
  - Game management endpoints
  - Save/load endpoints
- Build and package for Lambda (cargo-lambda)
- Deploy via CDK

**Phase 3: Frontend Integration**
- Update WASM app to call `api.gruesome.skeptomai.com`
- Add login UI components
- Implement JWT token management
- Replace local save/load with API calls
- Add save game browser UI
- Deploy to S3 via CDK

**Phase 4: Testing & Launch**
- End-to-end testing
- Load testing
- Security review
- Production deployment
- Redirect GitHub Pages to new domain

**Phase 5: Enhancements (Future)**
- Additional OAuth providers (Google, Discord)
- Achievement system
- Leaderboards
- Social features (friends, sharing)

### Infrastructure as Code Approach

**Selected Tool**: AWS CDK (TypeScript)

**Why CDK over alternatives:**
- **vs CloudFormation**: Higher-level abstractions, type-safe
- **vs Terraform**: Native AWS integration, better AWS service support
- **vs Serverless Framework**: More control, not Lambda-only
- **vs SAM**: More flexible, better for complex architectures

**CDK Benefits:**
1. **Zero manual clicking** in AWS console
2. **Type-safe** infrastructure definitions
3. **Reusable** constructs and patterns
4. **Version controlled** infrastructure
5. **Preview changes** before deployment (`cdk diff`)
6. **Rollback** support built-in
7. **Cross-stack references** handled automatically

**CDK Stack Structure:**
```
infrastructure/
├── bin/
│   └── gruesome-platform.ts       # CDK app entry point
├── lib/
│   ├── dns-stack.ts               # Route 53 + ACM certificate
│   ├── frontend-stack.ts          # S3 + CloudFront
│   ├── backend-stack.ts           # API Gateway + Lambda
│   ├── auth-stack.ts              # Cognito User Pool
│   └── data-stack.ts              # DynamoDB + S3 saves
├── lambda/
│   └── rust-functions/            # Rust Lambda code
│       ├── auth/
│       ├── games/
│       └── saves/
├── cdk.json
└── package.json
```

### Next Steps (Automated via CDK)

**See detailed implementation guides**:
- **[AWS CDK Implementation Plan](AWS_CDK_IMPLEMENTATION_PLAN.md)** - Complete infrastructure as code guide
- **[CDK Quick Start Checklist](CDK_QUICK_START_CHECKLIST.md)** - Step-by-step deployment checklist

**Quick summary**:
1. **Initialize CDK project** in repository
2. **Define all infrastructure** in TypeScript
3. **Deploy with single command**: `cdk deploy --all`
4. **Verify resources** created automatically
5. **Update DNS** (A records created by CDK)
6. **Deploy frontend** to S3 (via CDK or CI/CD)
7. **Test end-to-end** with real domain

**Key files created**:
- `infrastructure/bin/gruesome-platform.ts` - CDK app entry point
- `infrastructure/lib/dns-stack.ts` - Route 53 + ACM
- `infrastructure/lib/data-stack.ts` - DynamoDB + S3 saves
- `infrastructure/lib/auth-stack.ts` - Cognito User Pool
- `infrastructure/lib/backend-stack.ts` - API Gateway + Lambda
- `infrastructure/lib/frontend-stack.ts` - S3 + CloudFront
- `infrastructure/lambda/gruesome-api/` - Rust Lambda functions
