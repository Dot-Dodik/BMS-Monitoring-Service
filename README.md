# BMS — Borwita Monitoring Service (`cmk-enterprise-monitor`)

Sistem monitoring infrastruktur multi-cabang kustom pengganti Checkmk Server berskala hingga **3.000+ perangkat lintas cabang**, dibangun menggunakan **Cargo Workspace (Rust)** untuk Central Server & Agent, **MariaDB** untuk penyimpanan persisten, serta **React + Vite + TailwindCSS** untuk Dashboard Web.

---

## 1. Arsitektur Sistem

1. **`crates/monitoring-proto` (Shared Protocol & Security):**
   - Zero-IP Lock Identity berbasis `HostUUID` (`/etc/machine-id` di Linux, `MachineGuid` Registry di Windows, `ANDROID_ID` di Android) dan `HostName`.
   - Otentikasi 2-Fase: Enrollment Token per cabang (`POST /api/v1/register`) + tanda tangan `HMAC-SHA256` dengan proteksi anti-replay `±300 detik` (`POST /api/v1/ingest`).
   - Formatter dan parser kompatibel penuh dengan protokol Checkmk (`<<<check_mk>>>` dan `<<<local>>>`).
2. **`crates/agent-rust` & `cmkagent-rust` (Lightweight Rust Agents — Linux, Windows, Android):**
   - Binary ringan (< 4 MB, RAM < 8 MB) dengan profil rilis `opt-level = "z"`, `lto = true`, `strip = true`, `panic = "abort"`.
   - Mengeluarkan **hanya** daftar service yang telah dikurasi (tanpa *unmonitored service clutter*).
   - Mendukung mode Hybrid (TCP Pull port `6556` + HTTP Push dengan *Random Jitter ±5 detik* dan *In-Memory Ring Buffer* 50 paket saat koneksi cabang terputus) serta **Branch Proxy Poller** (SNMP / REST / Ping).
3. **`crates/server-rust` (Central Server — Optimasi 3.000 Perangkat):**
   - **In-Memory State Cache (`DashMap`) & Diff-Only Batch Write:** Menyimpan state real-time di RAM; jika status tetap `OK`, hanya melakukan *batch flush* ke MariaDB setiap **3 menit** (memangkas beban Disk I/O >85%). Jika terjadi *state transition* (`OK` -> `WARN`/`CRIT` atau `RECOVERY`), langsung ditulis ke MariaDB detik itu juga dan dikirim ke antrean notifikasi.
   - **Stale Host Detector:** Worker background setiap 30 detik yang menandai host sebagai `DOWN` jika tidak melapor lebih dari `180 detik`.
   - **Decoupled Notification Worker:**
     - **IT Ops Cabang:** Alert infrastruktur dikirim ke **Telegram Group Forum Topics** (`telegram_chat_id` + `telegram_topic_id` per cabang) dengan proteksi *flapping* (2x siklus berturut-turut).
     - **End-User / Pemilik Perangkat:** Pengingat ramah dikirim ke **Email Utama (SMTP)** dan **WhatsApp Webhook** khusus untuk service milik user (`Health_Battery`, `Storage_Usage`, `Printer_Paper`).
4. **`frontend/` (BMS Custom Dashboard):**
   - Navigasi ala Checkmk dengan kartu agregasi per cabang, tabel **All Hosts** (server-side pagination 50 baris/halaman), drill-down **1-per-1 Services of Host** lengkap dengan *Perf-O-Meter*, halaman **Service Detail** dengan grafik histori (`4h`, `25h`, `8d`, `35d`, `400d`), manajemen User RBAC, halaman **Settings** (REST API, SMTP Email Utama + Test Email, WhatsApp, Telegram, Jenis Device Dinamis), **Export Center (CSV & PDF)**, serta halaman **Download Agent (Windows, Linux, Android APK)**.

---

## 2. Spesifikasi Agent Linux, Windows & Android APK

Dokumentasi detail mengenai daftar service eksak, jadwal cache (Real-time 60 detik, Harian 16:00, Sabtu 11:00), perbaikan skrip Linux/Windows, serta fitur UI aplikasi Android (`EXPAND ALL` / `COLLAPSE ALL`, Expand/Collapse per menu, dan `START` / `STOP AGENT`) tersedia di repositori agen:
- Lihat [`cmkagent-rust/README.md`](../cmkagent-rust/README.md) untuk daftar lengkap service dan panduan instalasi agen di Linux, Windows, dan Android.

---

## 3. Cara Install & Deploy ke Container Server (Docker)

Secara default, instalasi baru akan memulai database **dalam keadaan kosong untuk daftar cabang dan host** sehingga Anda dapat menambahkannya secara manual melalui Dashboard Web BMS.

### Prasyarat Server
- **Docker Engine** & **Docker Compose v2** (`docker compose version`)
- **Git**

### Langkah Instalasi

```bash
# 1. Clone repository dari GitHub
git clone https://github.com/Dot-Dodik/BMS-Monitoring-Service.git
cd BMS-Monitoring-Service

# 2. Salin file environment contoh dan sesuaikan kredensial
cp .env.example .env
nano .env
```

Pastikan Anda mengganti nilai penting di `.env` sebelum menjalankan container:
- `MARIADB_ROOT_PASSWORD`, `MARIADB_PASSWORD`, dan `DATABASE_URL` (sesuaikan password database).
- `JWT_SECRET` dan `HMAC_MASTER_PEPPER` (ganti dengan string acak minimal 32 karakter).
- `DEFAULT_ADMIN_USERNAME` & `DEFAULT_ADMIN_PASSWORD` (kredensial admin awal).
- `DEFAULT_BRANCH_CODE` & `DEFAULT_BRANCH_NAME` biarkan **kosong** jika ingin menambahkan cabang dan host secara manual melalui Dashboard.
- *(Opsional)* Isi konfigurasi `TELEGRAM_BOT_TOKEN`, `WA_WEBHOOK_URL`, dan `SMTP_*` (atau atur langsung nanti dari menu **Settings** di Dashboard).

```bash
# 3. Siapkan folder binary agent (opsional jika menggunakan self-hosted download)
mkdir -p dist/agents

# 4. Build dan jalankan seluruh stack (MariaDB 11.4 + BMS Rust Server + Frontend Nginx)
docker compose up -d --build

# 5. Periksa status container & log server
docker compose ps
docker compose logs -f server-rust
```

### Akses Layanan & Penggunaan Awal
- **URL Dashboard Web BMS:** `http://<IP-SERVER>:3000`
- **URL REST API & Ingest Server:** `http://<IP-SERVER>:8080`
- **Healthcheck API:** `http://<IP-SERVER>:8080/healthz` (atau `http://<IP-SERVER>:3000/healthz`)
- **Input Cabang & Host Manual:**
  1. Buka Dashboard di `http://<IP-SERVER>:3000`.
  2. Buat cabang terlebih dahulu melalui tombol **`+ Tambah Cabang`** pada halaman **Overview Cabang**.
  3. Tambahkan perangkat melalui tombol **`+ Tambah Host Manual`** di sidebar, atau unduh/pasang agent dari menu **Download Agent** agar host terdaftar ke cabang yang sudah dibuat.
- **Self-Hosted Agent Downloads:** Letakkan binary hasil build (`bms-agent-windows-x86_64.exe`, `bms-agent-linux-x86_64`, `bms-agent-linux-aarch64`, `cmkagent-android.apk`) ke dalam folder `./dist/agents/` (ter-mount ke `/app/downloads` di container `server-rust`).

### Update / Redeploy Container
```bash
git pull origin main
docker compose up -d --build
```
