# Thai Smart Card Reader

อ่านบัตรประชาชนไทย ส่งข้อมูลผ่าน WebSocket เพื่อ integrate กับระบบ HIS Centrix (MedHIS)

Built with **Rust** + **egui** (native UI) + **axum** (WebSocket server)

---

## Features

- อ่านบัตรประชาชนไทยผ่าน PC/SC (USB card reader)
- Decode TIS-620 encoding สำหรับชื่อ/ที่อยู่ภาษาไทย
- แสดงรูปภาพบนบัตร (JPEG base64)
- WebSocket server ส่งข้อมูลแบบ real-time ที่ `ws://localhost:8182`
- Native UI (egui) แสดงข้อมูลบัตร พร้อม toggle EN/TH และ show/hide
- หน้าต่างขนาดคงที่ 1100×750 (non-resizable)
- รองรับ 2 ภาษา (ไทย/อังกฤษ) สลับได้ที่ status bar
- ข้อมูลซ่อนเป็น default กดปุ่มถึงจะแสดง
- Log แสดงเลขบัตรแบบ mask เฉพาะ 4 ตัวท้าย

---

## Architecture

```
[Smart Card] → [PC/SC] → [reader.rs] → [decoder.rs]
                                              │
                        ┌─────────────────────┤
                        ▼                     ▼
                  [server.rs]             [ui.rs]
              WebSocket :8182          Native egui UI
                        │
                        ▼
              {"mode":"readsmartcard", ...}
                        │
                        ▼
                  HIS Centrix
          (registration.controller.js)
```

---

## Project Structure

```
smart-card-reader/
├── backend/
│   ├── src/
│   │   ├── main.rs      # Entry point, WebSocket message format
│   │   ├── config.rs    # Configuration (port, window size, etc.)
│   │   ├── reader.rs    # PC/SC card reading + TIS-620 address parsing
│   │   ├── decoder.rs   # ThaiIDData struct + apply_output_config
│   │   ├── server.rs    # WebSocket server (axum)
│   │   └── ui.rs        # Native UI (egui), i18n EN/TH
│   ├── assets/
│   │   ├── flag_th.png  # ธงไทย (embedded)
│   │   └── flag_gb.png  # ธงอังกฤษ (embedded)
│   └── config.toml      # Configuration file
└── frontend/            # React + Vite (optional web UI)
    └── src/
        ├── types.ts
        ├── hooks/useCardReader.ts
        └── components/CardInfo.tsx
```

---

## Prerequisites

- **Rust** — [Install](https://www.rust-lang.org/tools/install)
- **PC/SC middleware**:
  - macOS: built-in (`SmartCardServices`)
  - Windows: built-in (`Smart Card` service)
  - Linux: `sudo apt install pcscd libpcsclite-dev && sudo systemctl start pcscd`
- Thai font (ดูหัวข้อ Font Support ด้านล่าง)

---

## Quick Start

```bash
cd smart-card-reader/backend
cargo run
```

หรือ build release:

```bash
cargo build --release
./target/release/smart-card-reader
```

---

## Configuration (`config.toml`)

```toml
[server]
host = "127.0.0.1"
port = 8182          # ต้องตรงกับ appConstant.MykadReaderUrl ใน HIS Centrix

[ui]
window_width  = 1100.0
window_height = 750.0
# window ไม่ resizable (min = max = initial)

[output]
include_photo = true

[logging]
level = "info"   # trace | debug | info | warn | error
```

---

## WebSocket API

**URL:** `ws://localhost:8182` (ไม่มี path `/ws`)

### Card Inserted

```json
{
  "mode": "readsmartcard",
  "Citizenid": "3100600123456",
  "Th_Prefix": "นาย",
  "Th_Firstname": "สมชาย",
  "Th_Middlename": "",
  "Th_Lastname": "ใจดี",
  "full_name_en": "Mr. Somchai Jaidee",
  "Birthday": "2520/04/13",
  "Sex": "1",
  "card_issuer": "ที่ว่าการอำเภอเมืองกรุงเทพมหานคร",
  "issue_date": "2566/03/01",
  "expire_date": "2576/04/12",
  "Address": "99 หมู่ที่ 4 ตำบลบางรัก อำเภอเมือง จังหวัดกรุงเทพมหานคร",
  "addrHouseNo": "99",
  "addrVillageNo": "หมู่ที่ 4",
  "addrTambol": "ตำบลบางรัก",
  "addrAmphur": "อำเภอเมือง",
  "PhotoRaw": "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8U..."
}
```

### Card Removed

```json
{
  "mode": "removedsmartcard"
}
```

### Field Reference

| Field | Description | Format |
|-------|-------------|--------|
| `Citizenid` | เลขบัตรประชาชน 13 หลัก | String |
| `Th_Prefix` | คำนำหน้า | String (Thai) |
| `Th_Firstname` | ชื่อ | String (Thai) |
| `Th_Middlename` | ชื่อกลาง (อาจว่าง) | String (Thai) |
| `Th_Lastname` | นามสกุล | String (Thai) |
| `full_name_en` | ชื่อ-นามสกุล ภาษาอังกฤษ | String |
| `Birthday` | วันเกิด (พ.ศ.) | `YYYY/MM/DD` |
| `Sex` | เพศ | `"1"` = ชาย, อื่นๆ = หญิง |
| `card_issuer` | หน่วยงานออกบัตร | String (Thai) |
| `issue_date` | วันออกบัตร (พ.ศ.) | `YYYY/MM/DD` |
| `expire_date` | วันหมดอายุ (พ.ศ.) | `YYYY/MM/DD` |
| `Address` | ที่อยู่รวม (house+village+tambol+amphur+province) | String (Thai) |
| `addrHouseNo` | เลขที่บ้าน | String |
| `addrVillageNo` | หมู่ที่ | String (Thai) |
| `addrTambol` | ตำบล/แขวง | String (Thai) |
| `addrAmphur` | อำเภอ/เขต | String (Thai) |
| `PhotoRaw` | รูปภาพบนบัตร | Base64 JPEG |

> **หมายเหตุ:** `Birthday`, `issue_date`, `expire_date` เป็น **ปี พ.ศ.** (Buddhist Era) format `YYYY/MM/DD`

---

## HIS Centrix Integration

HIS Centrix (MedHIS) connect WebSocket ที่ `appConstant.MykadReaderUrl = ws://localhost:8182`

```javascript
// registration.controller.js (line ~6865)
socketForMykad.onmessage = function (e) {
    var patientData = JSON.parse(e.data);
    if (patientData.mode == "readsmartcard") {
        searchpatientfromnationid(patientData.Citizenid, patientData, true)
    }
};
```

Fields ที่ HIS ใช้:
- `patientData.Citizenid` → ค้นหาผู้ป่วย
- `patientData.Th_Firstname/Th_Middlename/Th_Lastname` → กรอกชื่อ
- `patientData.Birthday` → parse ด้วย `moment(val, 'YYYY/MM/DD')`
- `patientData.addrTambol` → `.replace('ตำบล','')` แล้ว query `/framework/area/search`
- `patientData.addrAmphur` → `.replace('อำเภอ','')` แล้ว filter area
- `patientData.PhotoRaw` → `'data:image/jpeg;base64,' + val`

> **Port 8181** ใช้สำหรับ `ScannerUrl` (เครื่อง scan เอกสาร) — **ห้ามใช้ port นี้**

---

## Address Parsing

บัตรประชาชนไทยเก็บที่อยู่เป็น TIS-620 bytes คั่นด้วย `#`:

```
99#หมู่ที่ 4###ตำบลบางรัก#อำเภอเมือง#จังหวัดกรุงเทพมหานคร#[garbage bytes]
```

- parts ว่างๆ ระหว่าง delimiters ถูก filter ออก
- garbage bytes หลัง province ถูกตัดโดย whitelist Thai consonants/vowels เท่านั้น
- ตัวเลข/ASCII ใน house/village part ถูกเก็บไว้ตามปกติ

---

## Thai Font Support

ลำดับการค้นหา font:

1. `custom_paths` ใน `config.toml`
2. `fonts/NotoSansThai-Regular.ttf` (relative to executable)
3. Windows system fonts: Leelawadee UI, Tahoma, Cordia New
4. Linux: `/usr/share/fonts/.../NotoSansThai-Regular.ttf`
5. macOS: Silom, Ayuthaya, Krungthep, Sathu

Download: [Noto Sans Thai — Google Fonts](https://fonts.google.com/noto/specimen/Noto+Sans+Thai)

```
smart-card-reader   ← binary
fonts/
  └── NotoSansThai-Regular.ttf
```

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SMART_CARD_CONFIG` | path ของ config.toml | ค้นหาอัตโนมัติ |
| `RUST_LOG` | log level override | ใช้ค่าใน config.toml |

---

## Troubleshooting

### บัตรไม่ถูกอ่าน
- ตรวจสอบว่า driver ของ card reader ติดตั้งแล้ว
- Linux: `sudo systemctl start pcscd`
- ลอง `pcsc_scan` เพื่อดูว่า reader ถูกพบ

### WebSocket connect ไม่ได้
- ตรวจสอบว่า backend รันอยู่
- ตรวจสอบ port 8182 ไม่ถูก firewall block
- ดู log: `{isTrusted: true}` ใน console หมายถึง connect ล้มเหลว (port ผิดหรือ backend ไม่รัน)

### Thai text แสดงเป็นกล่องสี่เหลี่ยม
- ติดตั้ง font ใน `fonts/NotoSansThai-Regular.ttf` ข้างๆ binary
- ดู log `Thai font not found` เพื่อดู path ที่ค้นหา

### Debug logging
```toml
# config.toml
[logging]
level = "debug"
```
หรือ `RUST_LOG=debug cargo run`
