<script setup>
import { computed, onMounted, onUnmounted, ref, watch } from "vue";

const base = import.meta.env.BASE_URL;
const mobileOpen = ref(false);
const activeSection = ref("overview");
const savedLanguage = localStorage.getItem("usedns-docs-language");
const lang = ref(
  savedLanguage === "id" || savedLanguage === "en" ? savedLanguage : "en",
);

const content = {
  id: {
    title: "UseDNS - Kelola DNS dengan mudah",
    language: "Bahasa",
    languageToggle: "Ganti bahasa ke Inggris",
    openNavigation: "Buka navigasi",
    closeNavigation: "Tutup navigasi",
    download: "Unduh",
    navigation: [
      ["overview", "Ringkasan"],
      ["features", "Fitur utama"],
      ["providers", "Provider DNS"],
      ["installation", "Instalasi"],
      ["usage", "Cara penggunaan"],
      ["privacy", "Privasi & DoH"],
      ["tray", "System tray"],
      ["development", "Pengembangan"],
      ["faq", "FAQ"],
    ],
    hero: {
      slogan: "Pilih. Ganti. Terhubung.",
      title: "DNS yang tepat,",
      highlight: "tanpa langkah rumit.",
      text: "UseDNS adalah aplikasi desktop ringan untuk menemukan, membandingkan, dan menerapkan resolver DNS - lengkap dengan profil keamanan, DNS terenkripsi, statistik jaringan, dan system tray.",
      download: "Unduh versi terbaru",
      source: "Lihat source code",
      openSource: "Open source",
      dashboard: "Dasbor UseDNS",
      imageAlt: "Dasbor aplikasi UseDNS",
      note: "DNS aktif terlihat jelas!",
    },
    featuresIntro: [
      "01 - Yang bisa dilakukan",
      "Satu aplikasi untuk mengelola DNS.",
      "Dirancang agar pengaturan jaringan yang teknis terasa sederhana dan aman.",
    ],
    features: [
      [
        "switch",
        "Ganti DNS tanpa ribet",
        "Pilih provider dan profil, kemudian biarkan UseDNS menangani konfigurasi adapter Windows.",
      ],
      [
        "shield",
        "DNS over HTTPS",
        "Gunakan mode encrypted-only pada profil yang memiliki pasangan alamat dan template DoH yang valid.",
      ],
      [
        "pulse",
        "Monitor jaringan",
        "Pantau download, upload, status koneksi, serta perkiraan latensi resolver secara real-time.",
      ],
      [
        "tray",
        "Berjalan di system tray",
        "Tutup window utama tanpa menghentikan aplikasi, lalu buka atau ganti DNS dari tray.",
      ],
      [
        "profile",
        "Profil terkurasi",
        "Bandingkan resolver berdasarkan privasi, keamanan, keluarga, iklan, dan kebutuhan produktivitas.",
      ],
      [
        "local",
        "Data tetap lokal",
        "Preferensi dan custom DNS disimpan pada perangkat tanpa dikirim ke server UseDNS.",
      ],
    ].map(([icon, title, text]) => ({ icon, title, text })),
    providersIntro: [
      "02 - Pilih sesuai kebutuhan",
      "Provider terpercaya, profil yang mudah dipahami.",
      "Alih-alih hanya menampilkan alamat IP, UseDNS menjelaskan manfaat setiap resolver dan profilnya - mulai dari performa hingga perlindungan keluarga.",
      "Provider bawaan bersifat read-only agar konfigurasi terverifikasi tetap utuh. Anda tetap dapat membuat profil DNS kustom.",
    ],
    providers: [
      ["Cloudflare", "Cepat & privat", "DoH"],
      ["Google Public DNS", "Andal & global", "DoH"],
      ["Quad9", "Blokir ancaman", "DoH"],
      ["AdGuard DNS", "Blokir iklan", "DoH"],
      ["CleanBrowsing", "Filter keluarga", "DoH"],
      ["Control D", "Filter fleksibel", "DoH"],
      ["NextDNS", "Privasi cloud", "DNS"],
      ["OpenDNS", "Keamanan Cisco", "DoH"],
    ],
    installation: [
      "03 - Mulai menggunakan",
      "Pilih paket untuk perangkat Anda.",
      "Catatan dukungan platform",
      "UseDNS tersedia untuk Windows, Linux, dan macOS. Integrasi perubahan DNS sistem saat ini paling lengkap di Windows.",
      "Buka semua release →",
    ],
    installOptions: [
      {
        os: "Windows",
        badge: "Integrasi penuh",
        files: ".exe / .msi",
        command: "Unduh installer terbaru dari GitHub Releases",
        accent: true,
      },
      {
        os: "Linux",
        badge: "Tersedia",
        files: ".deb / .rpm / .tar.gz",
        command: "sudo apt install ./usedns_*_amd64.deb",
      },
      {
        os: "macOS",
        badge: "Tersedia",
        files: ".dmg / .zip",
        command: "Buka DMG lalu pindahkan UseDNS ke Applications",
      },
    ],
    usage: [
      "04 - Alur penggunaan",
      "Dari pilih hingga aktif dalam empat langkah.",
      "Build dari source",
      "# Build produksi",
    ],
    steps: [
      "Buka katalog DNS dan cari provider berdasarkan kebutuhan.",
      "Pilih profil perlindungan, IPv4, IPv6, atau keduanya.",
      "Pilih Encrypted jika profil mendukung DoH, lalu Apply DNS.",
      "Pantau provider aktif, status, dan kecepatan dari app atau tray.",
    ],
    privacy: [
      "05 - Privasi yang transparan",
      "UseDNS tidak membaca query DNS Anda.",
      "UseDNS bukan layanan resolver. Query dikirim langsung ke provider pilihan Anda. Aplikasi tidak merekam, menjual, atau mengunggah riwayat DNS.",
    ],
    privacyItems: [
      [
        "DNS over HTTPS",
        "Mode encrypted-only menggunakan metadata DoH per alamat tanpa fallback plaintext.",
      ],
      [
        "Penyimpanan lokal",
        "Bahasa, tema, preferensi tray, dan custom provider disimpan di perangkat.",
      ],
      [
        "Elevasi on-demand",
        "UAC hanya diminta ketika Windows perlu mengubah atau mereset DNS.",
      ],
    ],
    tray: [
      "06 - Selalu tersedia",
      "Tutup window, bukan aplikasinya.",
      "UseDNS tetap berjalan di belakang layar melalui system tray. Hover untuk melihat statistik, klik kiri untuk membuka kembali, atau gunakan menu untuk mengganti DNS dan keluar.",
    ],
    trayLabels: [
      "Kecepatan langsung",
      "DNS aktif",
      "Status & latensi",
      "Ganti cepat",
    ],
    development: [
      "07 - Dibangun secara terbuka",
      "Rust di balik layar, Slint di depan.",
      "Arsitektur memisahkan model provider, penyimpanan lokal, operasi sistem Windows, dan UI agar tetap mudah dipelihara.",
      "Baca spesifikasi proyek →",
    ],
    technologies: [
      ["Rust", "Core, worker, dan integrasi Windows"],
      ["Slint", "UI desktop native yang ringan"],
      ["PowerShell", "DnsClient dan konfigurasi DoH"],
      ["GitHub Actions", "Build EXE, MSI, DEB, RPM, DMG"],
    ],
    faqIntro: ["08 - Pertanyaan umum", "Yang perlu Anda ketahui."],
    faqs: [
      [
        "Apakah UseDNS menjalankan resolver sendiri?",
        "Tidak. Query DNS langsung ditangani provider yang Anda pilih. UseDNS hanya membantu mengonfigurasi DNS pada adapter perangkat.",
      ],
      [
        "Mengapa UAC muncul saat menerapkan DNS?",
        "Windows memerlukan izin Administrator untuk mengubah pengaturan DNS. Aplikasi utama tetap berjalan normal dan elevasi hanya diminta saat apply atau reset.",
      ],
      [
        "Apakah semua provider mendukung encrypted DNS?",
        "Tidak. Opsi Encrypted hanya ditampilkan ketika profil memiliki alamat dan template DoH yang dapat diterapkan oleh Windows.",
      ],
      [
        "Apakah menutup window menghentikan aplikasi?",
        "Tidak. UseDNS tetap berjalan di system tray. Gunakan menu Close/Tutup pada tray untuk benar-benar keluar.",
      ],
      [
        "Platform apa saja yang didukung?",
        "UseDNS tersedia untuk Windows, Linux, dan macOS. Fitur antarmuka tersedia lintas platform, sedangkan integrasi untuk menerapkan DNS sistem saat ini paling lengkap di Windows.",
      ],
    ],
    cta: [
      "Siap mencoba?",
      "Kelola DNS dengan lebih sederhana.",
      "Unduh UseDNS atau ikuti pengembangannya secara terbuka di GitHub.",
      "Unduh UseDNS",
    ],
    footerSlogan: "Pilih. Ganti. Terhubung.",
    issues: "Isu",
    license: "Lisensi MIT",
    logoAlt: "Logo UseDNS",
    providerLogoAlt: "Logo provider",
  },
  en: {
    title: "UseDNS - Manage DNS with ease",
    language: "Language",
    languageToggle: "Switch language to Indonesian",
    openNavigation: "Open navigation",
    closeNavigation: "Close navigation",
    download: "Download",
    navigation: [
      ["overview", "Overview"],
      ["features", "Key features"],
      ["providers", "DNS providers"],
      ["installation", "Installation"],
      ["usage", "How to use"],
      ["privacy", "Privacy & DoH"],
      ["tray", "System tray"],
      ["development", "Development"],
      ["faq", "FAQ"],
    ],
    hero: {
      slogan: "Choose. Switch. Connect.",
      title: "The right DNS,",
      highlight: "without the complexity.",
      text: "UseDNS is a lightweight desktop app for finding, comparing, and applying DNS resolvers - complete with security profiles, encrypted DNS, network statistics, and a system tray.",
      download: "Download latest version",
      source: "View source code",
      openSource: "Open source",
      dashboard: "UseDNS Dashboard",
      imageAlt: "UseDNS application dashboard",
      note: "See your active DNS at a glance!",
    },
    featuresIntro: [
      "01 - What it can do",
      "One app to manage your DNS.",
      "Designed to make technical network settings feel simple and secure.",
    ],
    features: [
      [
        "switch",
        "Switch DNS effortlessly",
        "Choose a provider and profile, then let UseDNS configure your Windows adapter.",
      ],
      [
        "shield",
        "DNS over HTTPS",
        "Use encrypted-only mode with profiles that have a valid address and DoH template pair.",
      ],
      [
        "pulse",
        "Network monitoring",
        "Monitor download, upload, connection status, and estimated resolver latency in real time.",
      ],
      [
        "tray",
        "Runs in the system tray",
        "Close the main window without stopping the app, then reopen it or switch DNS from the tray.",
      ],
      [
        "profile",
        "Curated profiles",
        "Compare resolvers by privacy, security, family protection, ad blocking, and productivity needs.",
      ],
      [
        "local",
        "Data stays local",
        "Preferences and custom DNS settings remain on your device and are never sent to UseDNS servers.",
      ],
    ].map(([icon, title, text]) => ({ icon, title, text })),
    providersIntro: [
      "02 - Choose what fits",
      "Trusted providers, easy-to-understand profiles.",
      "Instead of only showing IP addresses, UseDNS explains the benefits of every resolver and profile - from performance to family protection.",
      "Built-in providers are read-only to keep verified configurations intact. You can still create custom DNS profiles.",
    ],
    providers: [
      ["Cloudflare", "Fast & private", "DoH"],
      ["Google Public DNS", "Reliable & global", "DoH"],
      ["Quad9", "Threat blocking", "DoH"],
      ["AdGuard DNS", "Ad blocking", "DoH"],
      ["CleanBrowsing", "Family filtering", "DoH"],
      ["Control D", "Flexible filtering", "DoH"],
      ["NextDNS", "Cloud privacy", "DNS"],
      ["OpenDNS", "Cisco security", "DoH"],
    ],
    installation: [
      "03 - Get started",
      "Choose a package for your device.",
      "Platform support note",
      "UseDNS is available for Windows, Linux, and macOS. System DNS integration is currently most complete on Windows.",
      "View all releases →",
    ],
    installOptions: [
      {
        os: "Windows",
        badge: "Full integration",
        files: ".exe / .msi",
        command: "Download the latest installer from GitHub Releases",
        accent: true,
      },
      {
        os: "Linux",
        badge: "Available",
        files: ".deb / .rpm / .tar.gz",
        command: "sudo apt install ./usedns_*_amd64.deb",
      },
      {
        os: "macOS",
        badge: "Available",
        files: ".dmg / .zip",
        command: "Open the DMG, then move UseDNS to Applications",
      },
    ],
    usage: [
      "04 - How it works",
      "From selection to active in four steps.",
      "Build from source",
      "# Production build",
    ],
    steps: [
      "Open the DNS catalog and find a provider based on your needs.",
      "Choose a protection profile, IPv4, IPv6, or both.",
      "Select Encrypted if the profile supports DoH, then Apply DNS.",
      "Monitor the active provider, status, and speed from the app or tray.",
    ],
    privacy: [
      "05 - Transparent privacy",
      "UseDNS does not read your DNS queries.",
      "UseDNS is not a resolver service. Queries go directly to your selected provider. The app does not record, sell, or upload your DNS history.",
    ],
    privacyItems: [
      [
        "DNS over HTTPS",
        "Encrypted-only mode uses per-address DoH metadata without a plaintext fallback.",
      ],
      [
        "Local storage",
        "Language, theme, tray preferences, and custom providers are stored on your device.",
      ],
      [
        "On-demand elevation",
        "UAC is requested only when Windows needs to change or reset DNS.",
      ],
    ],
    tray: [
      "06 - Always available",
      "Close the window, not the app.",
      "UseDNS keeps running in the background through the system tray. Hover to see statistics, left-click to reopen it, or use the menu to switch DNS and exit.",
    ],
    trayLabels: [
      "Live speed",
      "Active DNS",
      "Status & latency",
      "Quick switch",
    ],
    development: [
      "07 - Built in the open",
      "Rust behind the scenes, Slint up front.",
      "The architecture separates provider models, local storage, Windows system operations, and the UI to keep everything maintainable.",
      "Read the project specification →",
    ],
    technologies: [
      ["Rust", "Core, workers, and Windows integration"],
      ["Slint", "Lightweight native desktop UI"],
      ["PowerShell", "DnsClient and DoH configuration"],
      ["GitHub Actions", "EXE, MSI, DEB, RPM, and DMG builds"],
    ],
    faqIntro: ["08 - Frequently asked questions", "What you need to know."],
    faqs: [
      [
        "Does UseDNS run its own resolver?",
        "No. DNS queries are handled directly by your selected provider. UseDNS only helps configure DNS on your device adapters.",
      ],
      [
        "Why does UAC appear when applying DNS?",
        "Windows requires Administrator permission to change DNS settings. The main app continues to run normally, and elevation is requested only when applying or resetting DNS.",
      ],
      [
        "Do all providers support encrypted DNS?",
        "No. The Encrypted option appears only when a profile has an address and DoH template that Windows can apply.",
      ],
      [
        "Does closing the window stop the app?",
        "No. UseDNS keeps running in the system tray. Use the Close option in the tray menu to exit completely.",
      ],
      [
        "Which platforms are supported?",
        "UseDNS is available for Windows, Linux, and macOS. The interface is cross-platform, while system DNS integration is currently most complete on Windows.",
      ],
    ],
    cta: [
      "Ready to try it?",
      "Manage DNS more simply.",
      "Download UseDNS or follow its open development on GitHub.",
      "Download UseDNS",
    ],
    footerSlogan: "Choose. Switch. Connect.",
    issues: "Issues",
    license: "MIT License",
    logoAlt: "UseDNS logo",
    providerLogoAlt: "Provider logo",
  },
};

const c = computed(() => content[lang.value]);
const navigation = computed(() =>
  c.value.navigation.map(([id, label]) => ({ id, label })),
);
const features = computed(() => c.value.features);
const providers = computed(() => c.value.providers);
const installOptions = computed(() => c.value.installOptions);
const faqs = computed(() => c.value.faqs);

watch(
  lang,
  (value) => {
    localStorage.setItem("usedns-docs-language", value);
    document.documentElement.lang = value;
    document.title = content[value].title;
  },
  { immediate: true },
);

function iconPath(name) {
  const paths = {
    switch: "M8 7h11m0 0-3-3m3 3-3 3M16 17H5m0 0 3-3m-3 3 3 3",
    shield:
      "M12 3 5 6v5c0 4.6 2.9 8.4 7 10 4.1-1.6 7-5.4 7-10V6l-7-3Zm-3 9 2 2 4-4",
    pulse: "M3 12h4l2-5 4 10 2-5h6",
    tray: "M5 5h14v10H5V5Zm-2 10h18v4H3v-4Zm6 2h6",
    profile: "M5 4h14v16H5V4Zm4 4h6M9 12h6M9 16h4",
    local:
      "M12 3a9 9 0 1 0 9 9M3 12h18M12 3c2.2 2.5 3.3 5.5 3 9-.3 3.5-1.3 6.5-3 9-1.7-2.5-2.7-5.5-3-9-.3-3.5.8-6.5 3-9Z",
  };
  return paths[name];
}

function providerLogo(name) {
  const logos = {
    Cloudflare: "cloudflare.svg",
    "Google Public DNS": "google.svg",
    Quad9: "quad9.svg",
    "AdGuard DNS": "adguard.svg",
    CleanBrowsing: "cleanbrowsing.svg",
    "Control D": "control-d.svg",
    NextDNS: "nextdns.svg",
    OpenDNS: "opendns.svg",
  };
  return `${base}providers/${logos[name]}`;
}

function scrollTo(id) {
  mobileOpen.value = false;
  document.getElementById(id)?.scrollIntoView({ behavior: "smooth" });
}

let observer;
onMounted(() => {
  observer = new IntersectionObserver(
    (entries) => {
      const visible = entries
        .filter((entry) => entry.isIntersecting)
        .sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];
      if (visible) activeSection.value = visible.target.id;
    },
    { rootMargin: "-20% 0px -60%", threshold: [0.1, 0.35, 0.65] },
  );
  navigation.value.forEach(({ id }) => {
    const section = document.getElementById(id);
    if (section) observer.observe(section);
  });
});
onUnmounted(() => observer?.disconnect());
</script>

<template>
  <div class="min-h-screen">
    <header
      class="sticky top-0 z-50 border-b border-blue-100/80 bg-white/90 backdrop-blur-xl"
    >
      <div
        class="mx-auto flex h-16 max-w-7xl items-center justify-between px-4 sm:px-6 lg:px-8"
      >
        <button class="flex items-center gap-3" @click="scrollTo('overview')">
          <img
            :src="`${base}usedns-logo.png`"
            :alt="c.logoAlt"
            class="h-10 w-10 rounded-xl shadow-sm"
          />
          <span class="text-lg font-extrabold tracking-tight text-navy"
            >UseDNS</span
          >
        </button>
        <nav class="hidden items-center gap-1 md:flex">
          <button
            v-for="item in navigation.slice(0, 5)"
            :key="item.id"
            class="rounded-lg px-3 py-2 text-sm font-semibold transition"
            :class="
              activeSection === item.id
                ? 'bg-blue-50 text-azure'
                : 'text-slate-600 hover:bg-slate-50 hover:text-navy'
            "
            @click="scrollTo(item.id)"
          >
            {{ item.label }}
          </button>
        </nav>
        <div class="flex items-center gap-2">
          <div
            class="flex rounded-lg border border-blue-100 bg-white p-0.5"
            role="group"
            :aria-label="c.language"
          >
            <button
              v-for="option in ['id', 'en']"
              :key="option"
              class="rounded-md px-2 py-1.5 text-xs font-bold uppercase transition"
              :class="
                lang === option
                  ? 'bg-blue-50 text-azure'
                  : 'text-slate-500 hover:text-navy'
              "
              :aria-pressed="lang === option"
              :aria-label="
                option === lang
                  ? `${c.language}: ${option.toUpperCase()}`
                  : c.languageToggle
              "
              @click="lang = option"
            >
              {{ option }}
            </button>
          </div>
          <a
            href="https://github.com/x-labs-myid/UseDNS/releases/latest"
            class="hidden rounded-xl bg-azure px-4 py-2.5 text-sm font-bold text-white shadow-lg shadow-blue-200 transition hover:-translate-y-0.5 hover:bg-blue-700 sm:inline-flex"
            >{{ c.download }}</a
          >
          <button
            class="grid h-10 w-10 place-items-center rounded-xl border border-blue-100 text-navy md:hidden"
            :aria-label="mobileOpen ? c.closeNavigation : c.openNavigation"
            @click="mobileOpen = !mobileOpen"
          >
            <svg
              class="h-5 w-5"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              viewBox="0 0 24 24"
            >
              <path
                :d="
                  mobileOpen
                    ? 'M6 6l12 12M18 6 6 18'
                    : 'M4 7h16M4 12h16M4 17h16'
                "
              />
            </svg>
          </button>
        </div>
      </div>
      <nav
        v-if="mobileOpen"
        class="border-t border-blue-50 bg-white p-3 shadow-xl md:hidden"
      >
        <button
          v-for="item in navigation"
          :key="item.id"
          class="block w-full rounded-lg px-4 py-2.5 text-left text-sm font-semibold text-slate-700 hover:bg-blue-50 hover:text-azure"
          @click="scrollTo(item.id)"
        >
          {{ item.label }}
        </button>
      </nav>
    </header>

    <main
      class="notebook-paper mx-auto min-h-screen max-w-[1440px] overflow-hidden border-x border-blue-100/80 shadow-2xl shadow-blue-200/40"
    >
      <section
        id="overview"
        class="section-anchor relative px-5 pb-24 pt-16 sm:px-10 lg:px-20 lg:pb-32 lg:pt-24"
      >
        <div
          class="absolute right-10 top-10 h-40 w-40 rounded-full bg-cyan/10 blur-3xl"
        ></div>
        <div
          class="mx-auto grid max-w-6xl items-center gap-14 lg:grid-cols-[1.05fr_.95fr]"
        >
          <div class="relative z-10">
            <div
              class="mb-6 inline-flex rotate-[-1deg] items-center gap-2 border border-cyan/30 bg-cyan/10 px-4 py-2 font-note text-lg text-blue-700 shadow-sm"
            >
              <span class="h-2 w-2 rounded-full bg-cyan"></span>
              {{ c.hero.slogan }}
            </div>
            <h1
              class="max-w-3xl text-5xl font-extrabold leading-[1.04] tracking-[-0.045em] text-navy sm:text-6xl lg:text-7xl"
            >
              {{ c.hero.title }}<br /><span class="text-azure">{{
                c.hero.highlight
              }}</span>
            </h1>
            <p class="mt-7 max-w-2xl text-lg leading-8 text-slate-600">
              {{ c.hero.text }}
            </p>
            <div class="mt-9 flex flex-wrap gap-3">
              <a
                href="https://github.com/x-labs-myid/UseDNS/releases/latest"
                class="inline-flex items-center gap-2 rounded-xl bg-azure px-5 py-3.5 font-bold text-white shadow-float transition hover:-translate-y-1 hover:bg-blue-700"
                >{{ c.hero.download }} <span>→</span></a
              >
              <a
                href="https://github.com/x-labs-myid/UseDNS"
                class="inline-flex items-center gap-2 rounded-xl border border-blue-200 bg-white px-5 py-3.5 font-bold text-navy transition hover:-translate-y-1 hover:border-azure hover:text-azure"
                >{{ c.hero.source }}</a
              >
            </div>
            <div
              class="mt-8 flex flex-wrap gap-x-6 gap-y-2 text-sm font-semibold text-slate-500"
            >
              <span>✓ {{ c.hero.openSource }}</span
              ><span>✓ Rust + Slint</span><span>✓ Windows · Linux · macOS</span>
            </div>
          </div>
          <div class="relative mx-auto w-full max-w-xl">
            <div
              class="absolute -inset-5 rotate-2 rounded-[2rem] border-2 border-dashed border-cyan/30"
            ></div>
            <div
              class="relative overflow-hidden rounded-2xl border border-blue-200 bg-white p-2 shadow-float"
            >
              <div class="flex items-center gap-1.5 px-3 py-2">
                <i class="h-2.5 w-2.5 rounded-full bg-red-300"></i
                ><i class="h-2.5 w-2.5 rounded-full bg-amber-300"></i
                ><i class="h-2.5 w-2.5 rounded-full bg-emerald-300"></i
                ><span class="ml-2 text-[11px] font-semibold text-slate-400">{{
                  c.hero.dashboard
                }}</span>
              </div>
              <img
                :src="`${base}dashboard.png`"
                :alt="c.hero.imageAlt"
                class="w-full rounded-xl border border-slate-100"
              />
            </div>
            <div
              class="absolute -bottom-8 -left-4 rotate-[-4deg] bg-yellow-100 px-4 py-3 font-note text-lg text-amber-900 shadow-note sm:-left-10"
            >
              {{ c.hero.note }}
            </div>
          </div>
        </div>
      </section>

      <section
        id="features"
        class="section-anchor border-t border-blue-100/70 px-5 py-24 sm:px-10 lg:px-20"
      >
        <div class="mx-auto max-w-6xl">
          <p class="font-note text-xl text-azure">{{ c.featuresIntro[0] }}</p>
          <div
            class="mt-2 flex flex-col justify-between gap-4 md:flex-row md:items-end"
          >
            <h2
              class="text-3xl font-extrabold tracking-tight text-navy sm:text-4xl"
            >
              {{ c.featuresIntro[1] }}
            </h2>
            <p class="max-w-md text-slate-500">
              {{ c.featuresIntro[2] }}
            </p>
          </div>
          <div class="mt-12 grid gap-5 md:grid-cols-2 lg:grid-cols-3">
            <article
              v-for="feature in features"
              :key="feature.title"
              class="note-card rounded-2xl p-6 transition duration-300 hover:-translate-y-1 hover:border-blue-300"
            >
              <div
                class="mb-5 grid h-11 w-11 place-items-center rounded-xl bg-blue-50 text-azure"
              >
                <svg
                  class="h-6 w-6"
                  fill="none"
                  stroke="currentColor"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="1.8"
                  viewBox="0 0 24 24"
                >
                  <path :d="iconPath(feature.icon)" />
                </svg>
              </div>
              <h3 class="text-lg font-bold text-navy">{{ feature.title }}</h3>
              <p class="mt-2 text-sm leading-6 text-slate-600">
                {{ feature.text }}
              </p>
            </article>
          </div>
        </div>
      </section>

      <section
        id="providers"
        class="section-anchor bg-blue-50/55 px-5 py-24 sm:px-10 lg:px-20"
      >
        <div class="mx-auto max-w-6xl">
          <div class="grid gap-12 lg:grid-cols-[.8fr_1.2fr]">
            <div>
              <p class="font-note text-xl text-azure">
                {{ c.providersIntro[0] }}
              </p>
              <h2
                class="mt-2 text-3xl font-extrabold tracking-tight text-navy sm:text-4xl"
              >
                {{ c.providersIntro[1] }}
              </h2>
              <p class="mt-5 leading-7 text-slate-600">
                {{ c.providersIntro[2] }}
              </p>
              <p
                class="mt-4 rounded-xl border-l-4 border-cyan bg-white p-4 text-sm leading-6 text-slate-600 shadow-sm"
              >
                {{ c.providersIntro[3] }}
              </p>
            </div>
            <div class="grid gap-3 sm:grid-cols-2">
              <div
                v-for="provider in providers"
                :key="provider[0]"
                class="flex items-center gap-4 rounded-xl border border-blue-100 bg-white p-4 shadow-sm transition hover:border-blue-300 hover:shadow-note"
              >
                <div
                  class="grid h-10 w-10 shrink-0 place-items-center rounded-full bg-gradient-to-br from-blue-50 to-cyan/10 p-2"
                >
                  <img
                    :src="providerLogo(provider[0])"
                    :alt="`${c.providerLogoAlt} ${provider[0]}`"
                    class="h-full w-full object-contain"
                    loading="lazy"
                  />
                </div>
                <div class="min-w-0">
                  <h3 class="truncate font-bold text-navy">
                    {{ provider[0] }}
                  </h3>
                  <p class="text-xs text-slate-500">{{ provider[1] }}</p>
                </div>
                <span
                  class="ml-auto rounded-full px-2 py-1 text-[10px] font-bold"
                  :class="
                    provider[2] === 'DoH'
                      ? 'bg-cyan/10 text-cyan-700'
                      : 'bg-slate-100 text-slate-500'
                  "
                  >{{ provider[2] }}</span
                >
              </div>
            </div>
          </div>
        </div>
      </section>

      <section
        id="installation"
        class="section-anchor px-5 py-24 sm:px-10 lg:px-20"
      >
        <div class="mx-auto max-w-6xl">
          <p class="font-note text-xl text-azure">{{ c.installation[0] }}</p>
          <h2
            class="mt-2 text-3xl font-extrabold tracking-tight text-navy sm:text-4xl"
          >
            {{ c.installation[1] }}
          </h2>
          <div class="mt-10 grid gap-5 lg:grid-cols-3">
            <article
              v-for="item in installOptions"
              :key="item.os"
              class="rounded-2xl border p-6"
              :class="
                item.accent
                  ? 'border-azure bg-navy text-white shadow-float'
                  : 'border-blue-100 bg-white text-navy shadow-note'
              "
            >
              <div class="flex items-center justify-between">
                <h3 class="text-xl font-extrabold">{{ item.os }}</h3>
                <span
                  class="rounded-full px-2.5 py-1 text-[10px] font-bold uppercase tracking-wider"
                  :class="
                    item.accent
                      ? 'bg-cyan/20 text-cyan-200'
                      : 'bg-slate-100 text-slate-500'
                  "
                  >{{ item.badge }}</span
                >
              </div>
              <p class="mt-8 text-2xl font-bold">{{ item.files }}</p>
              <p
                class="mt-2 min-h-12 text-sm leading-6"
                :class="item.accent ? 'text-blue-100' : 'text-slate-500'"
              >
                {{ item.command }}
              </p>
            </article>
          </div>
          <div
            class="mt-8 flex flex-col items-start justify-between gap-4 rounded-2xl border border-blue-200 bg-blue-50 p-5 sm:flex-row sm:items-center"
          >
            <div>
              <p class="font-bold text-navy">{{ c.installation[2] }}</p>
              <p class="mt-1 text-sm text-slate-600">
                {{ c.installation[3] }}
              </p>
            </div>
            <a
              href="https://github.com/x-labs-myid/UseDNS/releases"
              class="shrink-0 font-bold text-azure hover:underline"
              >{{ c.installation[4] }}</a
            >
          </div>
        </div>
      </section>

      <section
        id="usage"
        class="section-anchor bg-navy px-5 py-24 text-white sm:px-10 lg:px-20"
      >
        <div class="mx-auto max-w-6xl">
          <p class="font-note text-xl text-cyan">{{ c.usage[0] }}</p>
          <h2 class="mt-2 text-3xl font-extrabold tracking-tight sm:text-4xl">
            {{ c.usage[1] }}
          </h2>
          <div class="mt-12 grid gap-8 md:grid-cols-2 lg:grid-cols-4">
            <div
              v-for="(step, index) in c.steps"
              :key="step"
              class="relative border-t border-blue-400/40 pt-6"
            >
              <span
                class="absolute -top-5 left-0 grid h-10 w-10 place-items-center rounded-full border-4 border-navy bg-cyan font-extrabold text-navy"
                >{{ index + 1 }}</span
              >
              <p class="mt-4 text-sm leading-7 text-blue-100">{{ step }}</p>
            </div>
          </div>
          <div
            class="mt-14 overflow-hidden rounded-2xl border border-blue-400/25 bg-[#0b1d38]"
          >
            <div
              class="flex items-center gap-2 border-b border-blue-400/20 px-5 py-3 text-xs text-blue-200"
            >
              <span class="h-2 w-2 rounded-full bg-cyan"></span>
              {{ c.usage[2] }}
            </div>
            <pre
              class="code-block overflow-x-auto p-5 text-sm leading-7 text-cyan"
            ><code>git clone https://github.com/x-labs-myid/UseDNS.git
cd UseDNS
cargo run

{{ c.usage[3] }}
cargo build --release</code></pre>
          </div>
        </div>
      </section>

      <section id="privacy" class="section-anchor px-5 py-24 sm:px-10 lg:px-20">
        <div class="mx-auto grid max-w-6xl gap-12 lg:grid-cols-2">
          <div>
            <p class="font-note text-xl text-azure">
              {{ c.privacy[0] }}
            </p>
            <h2
              class="mt-2 text-3xl font-extrabold tracking-tight text-navy sm:text-4xl"
            >
              {{ c.privacy[1] }}
            </h2>
            <p class="mt-5 leading-7 text-slate-600">
              {{ c.privacy[2] }}
            </p>
          </div>
          <div class="space-y-4">
            <div
              v-for="item in c.privacyItems"
              :key="item[0]"
              class="rounded-xl border border-blue-100 bg-white p-5 shadow-note"
            >
              <h3 class="font-bold text-navy">{{ item[0] }}</h3>
              <p class="mt-1 text-sm leading-6 text-slate-600">{{ item[1] }}</p>
            </div>
          </div>
        </div>
      </section>

      <section
        id="tray"
        class="section-anchor bg-gradient-to-br from-blue-50 to-cyan/10 px-5 py-24 sm:px-10 lg:px-20"
      >
        <div class="mx-auto max-w-6xl">
          <div class="note-card rounded-3xl p-8 sm:p-12">
            <div class="grid items-center gap-10 lg:grid-cols-[1fr_.8fr]">
              <div>
                <p class="font-note text-xl text-azure">{{ c.tray[0] }}</p>
                <h2
                  class="mt-2 text-3xl font-extrabold tracking-tight text-navy sm:text-4xl"
                >
                  {{ c.tray[1] }}
                </h2>
                <p class="mt-5 leading-7 text-slate-600">
                  {{ c.tray[2] }}
                </p>
              </div>
              <div class="grid grid-cols-2 gap-3">
                <div
                  v-for="label in c.trayLabels"
                  :key="label"
                  class="rounded-xl border border-blue-100 bg-blue-50/70 p-4 text-center text-sm font-bold text-blue-800"
                >
                  {{ label }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section
        id="development"
        class="section-anchor px-5 py-24 sm:px-10 lg:px-20"
      >
        <div class="mx-auto max-w-6xl">
          <p class="font-note text-xl text-azure">
            {{ c.development[0] }}
          </p>
          <div class="mt-2 grid gap-10 lg:grid-cols-[.8fr_1.2fr]">
            <div>
              <h2
                class="text-3xl font-extrabold tracking-tight text-navy sm:text-4xl"
              >
                {{ c.development[1] }}
              </h2>
              <p class="mt-5 leading-7 text-slate-600">
                {{ c.development[2] }}
              </p>
              <a
                href="https://github.com/x-labs-myid/UseDNS/blob/main/docs/spec/MVP_SPECIFICATION.md"
                class="mt-5 inline-block font-bold text-azure hover:underline"
                >{{ c.development[3] }}</a
              >
            </div>
            <div class="grid gap-3 sm:grid-cols-2">
              <div
                v-for="item in c.technologies"
                :key="item[0]"
                class="rounded-xl border border-blue-100 bg-white p-5 shadow-sm"
              >
                <p class="font-extrabold text-navy">{{ item[0] }}</p>
                <p class="mt-1 text-sm text-slate-500">{{ item[1] }}</p>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section
        id="faq"
        class="section-anchor border-t border-blue-100 px-5 py-24 sm:px-10 lg:px-20"
      >
        <div class="mx-auto max-w-4xl">
          <div class="text-center">
            <p class="font-note text-xl text-azure">{{ c.faqIntro[0] }}</p>
            <h2
              class="mt-2 text-3xl font-extrabold tracking-tight text-navy sm:text-4xl"
            >
              {{ c.faqIntro[1] }}
            </h2>
          </div>
          <div
            class="mt-10 divide-y divide-blue-100 overflow-hidden rounded-2xl border border-blue-100 bg-white shadow-note"
          >
            <details
              v-for="(faq, index) in faqs"
              :key="faq[0]"
              class="group p-5"
              :open="index === 0"
            >
              <summary
                class="flex cursor-pointer list-none items-center justify-between gap-5 font-bold text-navy"
              >
                <span>{{ faq[0] }}</span
                ><span
                  class="text-xl text-azure transition group-open:rotate-45"
                  >+</span
                >
              </summary>
              <p class="mt-3 pr-8 text-sm leading-6 text-slate-600">
                {{ faq[1] }}
              </p>
            </details>
          </div>
        </div>
      </section>

      <section class="px-5 pb-24 sm:px-10 lg:px-20">
        <div
          class="mx-auto max-w-6xl overflow-hidden rounded-3xl bg-gradient-to-r from-azure to-blue-600 p-8 text-white shadow-float sm:p-12"
        >
          <div
            class="flex flex-col items-start justify-between gap-7 md:flex-row md:items-center"
          >
            <div>
              <p class="font-note text-xl text-cyan-100">{{ c.cta[0] }}</p>
              <h2 class="mt-1 text-3xl font-extrabold">
                {{ c.cta[1] }}
              </h2>
              <p class="mt-3 max-w-xl text-blue-100">
                {{ c.cta[2] }}
              </p>
            </div>
            <a
              href="https://github.com/x-labs-myid/UseDNS/releases/latest"
              class="shrink-0 rounded-xl bg-white px-6 py-3.5 font-bold text-azure shadow-lg transition hover:-translate-y-1"
              >{{ c.cta[3] }}</a
            >
          </div>
        </div>
      </section>
    </main>

    <footer class="border-t border-blue-100 bg-white">
      <div
        class="mx-auto flex max-w-7xl flex-col gap-5 px-5 py-9 text-sm text-slate-500 sm:flex-row sm:items-center sm:justify-between"
      >
        <div class="flex items-center gap-3">
          <img
            :src="`${base}usedns-logo.png`"
            :alt="c.logoAlt"
            class="h-8 w-8 rounded-lg"
          /><span
            ><strong class="text-navy">UseDNS</strong> ·
            {{ c.footerSlogan }}</span
          >
        </div>
        <div class="flex gap-5">
          <a
            href="https://github.com/x-labs-myid/UseDNS"
            class="hover:text-azure"
            >GitHub</a
          ><a
            href="https://github.com/x-labs-myid/UseDNS/issues"
            class="hover:text-azure"
            >{{ c.issues }}</a
          ><a
            href="https://github.com/x-labs-myid/UseDNS/blob/main/LICENSE"
            class="hover:text-azure"
            >{{ c.license }}</a
          >
        </div>
      </div>
    </footer>
  </div>
</template>
