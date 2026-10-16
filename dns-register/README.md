<details open>
<summary><b>English Guide</b></summary>

# DNS Registration Guide (UseDNS)

Thank you for helping us expand the DNS list for UseDNS! This guide explains how to add new DNS providers or update existing ones.

## File Structure

- **dns.json**: The main database containing all DNS information.
- **icons/**: A directory for SVG icons. Every DNS provider must have an icon here.

## How to Add a New DNS Provider

### 1. Prepare the Icon

- **Format**: Must be a clean **SVG** file.
- **Requirements**:
  - Keep the code minimalist. Remove unnecessary metadata (like editor tags).
  - Use a proper `viewBox` to ensure it scales correctly.
  - Avoid fixed pixel widths/heights if possible; `1em` is preferred for flexibility.
- **Location**: Save it as `icons/[provider-id].svg`.

### 2. Update `dns.json`

Add a new entry to the array in `dns.json`. Follow this structure:

<details>
<summary>Click to view JSON structure</summary>

```json
{
  "id": "unique-provider-id",
  "name": "Display Name",
  "owner": "Company or Organization Name",
  "ipv4_primary": "1.1.1.1",
  "ipv4_secondary": "1.0.0.1",
  "ipv6_primary": "2606:4700:4700::1111",
  "ipv6_secondary": "2606:4700:4700::1001",
  "summary_en": "Short description in English",
  "summary_id": "Deskripsi singkat dalam Bahasa Indonesia",
  "purpose": "general | privacy | security | ad-blocking",
  "pros_en": "Key benefits (EN)",
  "pros_id": "Kelebihan utama (ID)",
  "cons_en": "Main drawbacks (EN)",
  "cons_id": "Kekurangan utama (ID)",
  "icon": "icons/filename.svg",
  "profiles": [
    {
      "id": "standard",
      "name_en": "Standard",
      "name_id": "Standar",
      "description_en": "Profile description",
      "description_id": "Deskripsi profil",
      "tag_en": "RECOMMENDED",
      "tag_id": "REKOMENDASI",
      "ipv4_primary": "1.1.1.1",
      "ipv4_secondary": "1.0.0.1",
      "ipv6_primary": "2606:4700:4700::1111",
      "ipv6_secondary": "2606:4700:4700::1001",
      "doh_template": "https://doh.example.com/dns-query"
    }
  ]
}
```

</details>

### 3. Key Rules

- **ID**: Use lowercase letters, numbers, and hyphens only.
- **Purpose**: Choose one that fits best: `general`, `privacy`, `security`, or `ad-blocking`.
- **Profiles**: If the provider offers different filters (e.g., family vs. security), list them all.
- **DoH template**: `doh_template` is optional and may be left out. Add it when the resolver does not answer plain DNS on port 53 (LibreDNS, for example, only speaks DoH and DoT), otherwise it must start with `https://`.

## How Data is Loaded

The app automatically fetches the latest data from these CDNs (in this order):

1. jsDelivr
2. Statically
3. GitHub Raw
4. Local Backup (if offline)

Just push your changes to the `main` branch, and the app will pick them up automatically!

</details>

---

<details>
<summary><b>Panduan Bahasa Indonesia</b></summary>

# Panduan Pendaftaran DNS (UseDNS)

Terima kasih sudah membantu memperkaya daftar DNS di UseDNS! Panduan ini menjelaskan cara menambah atau memperbarui data DNS.

## Struktur File

- **dns.json**: Database utama yang berisi semua informasi DNS.
- **icons/**: Folder untuk ikon SVG. Setiap provider DNS wajib punya ikon di sini.

## Cara Menambahkan DNS Baru

### 1. Siapkan Ikon

- **Format**: Wajib file **SVG**.
- **Ketentuan**:
  - Kode harus bersih dan minimalis. Hapus metadata yang tidak perlu.
  - Gunakan `viewBox` agar ikon bisa di-scale dengan benar.
  - Hindari ukuran pixel tetap (width/height); gunakan `1em` agar lebih fleksibel.
- **Lokasi**: Simpan dengan nama `icons/[provider-id].svg`.

### 2. Update `dns.json`

Tambahkan entri baru ke dalam array di `dns.json`. Gunakan struktur berikut:

<details>
<summary>Klik untuk melihat struktur JSON</summary>

```json
{
  "id": "id-provider-unik",
  "name": "Nama Tampilan",
  "owner": "Nama Perusahaan atau Organisasi",
  "ipv4_primary": "1.1.1.1",
  "ipv4_secondary": "1.0.0.1",
  "ipv6_primary": "2606:4700:4700::1111",
  "ipv6_secondary": "2606:4700:4700::1001",
  "summary_en": "Short description in English",
  "summary_id": "Deskripsi singkat dalam Bahasa Indonesia",
  "purpose": "general | privacy | security | ad-blocking",
  "pros_en": "Key benefits (EN)",
  "pros_id": "Kelebihan utama (ID)",
  "cons_en": "Main drawbacks (EN)",
  "cons_id": "Kekurangan utama (ID)",
  "icon": "icons/namafile.svg",
  "profiles": [
    {
      "id": "standard",
      "name_en": "Standard",
      "name_id": "Standar",
      "description_en": "Profile description",
      "description_id": "Deskripsi profil",
      "tag_en": "RECOMMENDED",
      "tag_id": "REKOMENDASI",
      "ipv4_primary": "1.1.1.1",
      "ipv4_secondary": "1.0.0.1",
      "ipv6_primary": "2606:4700:4700::1111",
      "ipv6_secondary": "2606:4700:4700::1001",
      "doh_template": "https://doh.example.com/dns-query"
    }
  ]
}
```

</details>

### 3. Aturan Penting

- **ID**: Gunakan huruf kecil, angka, dan tanda hubung saja.
- **Purpose**: Pilih yang paling sesuai: `general`, `privacy`, `security`, atau `ad-blocking`.
- **Profiles**: Jika provider punya beberapa filter (misal: untuk keluarga vs keamanan), masukkan semuanya di sini.
- **DoH template**: `doh_template` bersifat opsional dan boleh tidak ditulis. Isi hanya jika resolver tidak melayani DNS biasa di port 53 (contohnya LibreDNS yang hanya menyediakan DoH dan DoT), dan nilainya wajib diawali `https://`.

## Mekanisme Pemuatan Data

Aplikasi akan mengambil data terbaru dari CDN berikut secara otomatis:

1. jsDelivr
2. Statically
3. GitHub Raw
4. Backup Lokal (jika offline)

Cukup lakukan _push_ perubahan Anda ke branch `main`, dan aplikasi akan memperbarui datanya sendiri!

</details>
