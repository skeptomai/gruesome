# Self-hosted Gruesome platform

A drop-in replacement for the AWS serverless backend (`infrastructure/`), so the
Gruesome web platform can run on your own Kubernetes cluster (or any host) with no
AWS account. The Z-Machine interpreter runs **client-side as WASM**, so this is
purely a storage + auth service.

## What it replaces

| AWS (`infrastructure/`) | Self-hosted (`selfhost/`) |
|---|---|
| Cognito (accounts) | `users` table + Argon2 password hashing + self-issued JWT |
| Lambda + API Gateway | one long-running `axum` server (`selfhost/server`) |
| DynamoDB | SQLite (single file on a volume) |
| S3 (games + saves) | MinIO (S3-compatible; same presigned-URL flow) |
| CloudFront + S3 static site | the same server serves the SPA (same origin) |

It implements the **identical REST contract** the frontend expects
(`/api/auth/*`, `/api/games/*`, `/api/saves/*`, `/api/admin/*`), so `frontend/`
works unchanged aside from `dev-config.js` (now same-origin).

**Security note:** the AWS Lambdas disabled JWT signature verification
(`insecure_disable_signature_validation`), so any forged token was accepted. This
server verifies real HS256 signatures. Set a strong `JWT_SECRET`.

## Layout

```
selfhost/
  server/          axum service (src/{main,config,db,auth,games,saves,admin,s3,error}.rs)
    migrations/    SQLite schema (compiled into the binary via sqlx::migrate!)
  Remfile          container image (server binary + the static frontend)
  k8s/             Deployment / Service / Ingress / PVC / Secret template
```

## Configuration (env)

| Var | Default | Purpose |
|---|---|---|
| `BIND_ADDR` | `0.0.0.0:8080` | listen address |
| `DATABASE_URL` | `sqlite://gruesome.db` | SQLite path (`sqlite:///data/gruesome.db` in k8s) |
| `FRONTEND_DIR` | *(unset)* | dir of static SPA files to serve at `/` |
| `JWT_SECRET` | *(dev default — CHANGE)* | HS256 signing secret |
| `ACCESS_TOKEN_TTL_SECS` / `REFRESH_TOKEN_TTL_SECS` | 3600 / 30d | token lifetimes |
| `S3_ENDPOINT` | `http://localhost:9000` | MinIO endpoint |
| `S3_ACCESS_KEY` / `S3_SECRET_KEY` | `minioadmin` | MinIO credentials |
| `S3_REGION` | `us-east-1` | any string (MinIO ignores it) |
| `GAMES_BUCKET` / `SAVES_BUCKET` | `gruesome-games` / `gruesome-saves` | buckets |
| `PRESIGN_TTL_SECS` | 300 | presigned-URL lifetime |
| `CORS_ORIGIN` | `*` | SPA origin (unused when same-origin) |

## Build (with pelagos)

```bash
# from the repo root (context must include selfhost/ and frontend/)
pelagos build -t <registry>/gruesome-platform:<tag> --file selfhost/Remfile .
pelagos image push <registry>/gruesome-platform:<tag>
```

## Deploy (Kubernetes)

1. Stand up MinIO and create the `gruesome-games` + `gruesome-saves` buckets. Set
   a CORS policy on the saves bucket allowing GET/PUT/DELETE from the SPA origin.
2. Create the secret from `k8s/secret.example.yaml` (real `JWT_SECRET` +
   MinIO keys).
3. Fill the `__PLACEHOLDERS__` in `k8s/deploy.yaml` (registry/tag, `__HOST__`,
   `__MINIO_HOST__`) and `kubectl apply -f k8s/deploy.yaml`.
4. Point your home DNS `__HOST__` at the Traefik LoadBalancer IP.

## Bootstrap the first admin

There is (deliberately) no self-service admin endpoint. Sign up normally, then flip
the role in SQLite:

```sql
UPDATE users SET role = 'admin' WHERE username = '<you>';
```

(e.g. `kubectl exec … -- sqlite3 /data/gruesome.db "UPDATE users SET role='admin' WHERE username='cb'"`).
Then re-login and use `/api/admin/*` (or the admin UI) to upload games:
`POST /api/admin/games/upload-url` → PUT the `.z3/.z5` file to the URL →
`POST /api/admin/games` with the metadata.

## Local development

```bash
cd selfhost/server
DATABASE_URL=sqlite:///tmp/g.db JWT_SECRET=dev FRONTEND_DIR=../../frontend \
  S3_ENDPOINT=http://localhost:9000 cargo run
# → http://localhost:8080  (SPA + API on the same origin)
```
