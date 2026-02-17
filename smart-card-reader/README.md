# Thai Smart Card Reader — Sub-project

ดู [README หลัก](../README.md) สำหรับ documentation ครบถ้วน

## Quick Start

### Backend (Rust)

```bash
cd backend
cargo run
```

WebSocket จะรันที่ `ws://localhost:8182`

### Frontend (React) — optional

```bash
cd frontend
npm install
npm run dev
```

เปิด browser ที่ `http://localhost:5173`

## Build Production

```bash
# Backend
cd backend
cargo build --release

# Frontend
cd frontend
npm run build
```
