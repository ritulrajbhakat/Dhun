# DHUN — Your Party. Your DJ.

> **Local-First, Standalone DJ Party Application with Dedicated Real-Time Audio Engine, Deterministic Algorithmic Mixing, and Local Mobile QR Guest Requests.**

---

## 1. Executive Summary & Core Value Proposition

**DHUN** enables a non-professional host to run an entire wedding celebration, sangeet, garba, house party, or club night from their laptop without needing to manually DJ:
1. Open DHUN on the laptop.
2. Select DJ Level (Level 1: Pro 2-Deck, Level 2: Smart Mixing, Level 3: Advanced Performance, Level 4: Creative DJ Tools).
3. Select Indian or International Party Mood (Shaadi, Baraat, Mehendi, Haldi, Sangeet, Desi Club, Garba, Dandiya, Retro Bollywood, etc.).
4. Select music library and audio output interface.
5. Press **Start Party**.
6. DHUN continuously selects, beatmatches, aligns downbeats, and mixes music autonomously.
7. Displays a local Wi-Fi QR code that guests scan from their mobile phones.
8. Guests search the local catalog and submit song requests without installing any application.
9. Host approves, prioritizes, or triggers immediate "Force Next" overrides.
10. Audio playback runs in an isolated, high-priority native Rust audio thread that continues uninterrupted even if the host UI refreshes, crashes, or guest Wi-Fi drops.

---

## 2. Non-Negotiable Architecture Principles

- **Local-First & Offline**: Zero mandatory cloud infrastructure, external AI APIs, or streaming subscriptions.
- **Deterministic Signal Processing**: Algorithmic autocorrelation BPM detection, Constant-Q / chromagram Camelot key estimation, 16/32-bar phrase segmentation, and rule-based energy curves.
- **Audio Thread Isolation**: Real-time CPAL audio thread runs with zero memory allocations, zero disk I/O, zero database locks, and lock-free SPSC communication channels.
- **Host Override Supremacy**: Host retains full priority with manual mixing controls and explicit Force Next overrides.
- **Rolling Audio Cache**: Preloads active + 2 upcoming tracks, maintains a 10-track rolling window protected by active deck locks, and clears temporary cache after 3 hours of inactivity while strictly preserving permanent SQLite metadata.

---

## 3. Technology Stack

| Layer | Technologies |
| :--- | :--- |
| **Desktop Wrapper** | Tauri 2.x (Multi-platform desktop runtime, minimal memory footprint) |
| **Frontend UI** | React 18, TypeScript, Vite, Tailwind CSS, Lucide Icons, Zustand |
| **Backend & Audio Engine** | Rust 2021, CPAL (Native Audio I/O), Symphonia (Lossless multi-format decoder) |
| **DSP Mixing Chain** | Direct Form II Transposed Biquads, 3-Band Isolator EQ, Resonant Filter, 1ms Lookahead Brickwall Limiter |
| **Local Database** | SQLite 3 (WAL mode, versioned migrations, 24 normalized entities) |
| **Guest Server** | Axum (Embedded HTTP REST & WebSockets), QR SVG Generator, Token-bucket rate limiting |

---

## 4. Four Selectable DJ Levels

1. **Level 1 — Professional Two-Deck Mixer**: Virtual jog wheels, pitch control (+/-8% to +/-16%), manual BPM nudging, 3-band EQ isolators with kill switches, crossfader, headphone cue, cue points, and safe continuous background auto-play.
2. **Level 2 — Smart Mixing (Deterministic Algorithmic DJ)**: Automated BPM detection, downbeat phase-locking, Camelot wheel harmonic compatibility (+/-1 hour or relative major/minor swap), smart bass swapping (prevents low-end frequency collisions), and energy curve progression.
3. **Level 3 — Advanced Performance**: 4 virtual decks, hot cue juggling (8 color-coded pads per deck), beat loops (1, 2, 4, 8, 16 beats), resonant state-variable HPF/LPF sweep, and MIDI hardware mapping.
4. **Level 4 — Creative DJ Tools**: Live mashup tools, phrase-based transitions, wordplay transition assistance, and vinyl scratching emulation.

---

## 5. Indian Party Mood Profiles

| Mood | BPM Range | Primary Genres | Transition Preset | Energy Profile |
| :--- | :--- | :--- | :--- | :--- |
| **Shaadi** | 100 - 130 | Bollywood (50%), Punjabi (30%), Wedding Remixes (20%) | 16-bar Smart Bass Swap | Steady Wave |
| **Baraat** | 126 - 145 | Bhangra Dhol (45%), Punjabi Pop (35%), Brass Band (20%) | 8-bar Snappy Cut / Bass Swap | Peak Party (Sustained High) |
| **Mehendi** | 85 - 110 | Folk Bollywood (50%), Sufi (30%), Acoustic (20%) | 32-bar Equal Power Fade | Gentle Build-up |
| **Haldi** | 95 - 118 | Upbeat Traditional Folk (60%), Filmi Acoustic (40%) | 16-bar Smooth Crossfade | Steady |
| **Sangeet** | 115 - 135 | Choreography Anthems (60%), Punjabi Dance (40%) | 16-bar Filter & Bass Swap | Dynamic Waves |
| **Desi Club** | 124 - 132 | Commercial Desi Remix (50%), Tech House (30%), EDM (20%) | 32-bar Filter Sweep & Drop | Sustained Peak |
| **Garba** | 110 - 140 | Garba Folk (60%), Sanedo (40%) | Continuous Cyclic Rhythm Sync | Progressive Acceleration |
| **Dandiya** | 120 - 144 | Dandiya Raas (70%), Fusion Beats (30%) | Synchronized Beat-Lock Cut | Fast Continuous |
| **Retro Bollywood** | 90 - 125 | RD Burman Disco Funk (60%), Golden Classics (40%) | 16-bar Linear Fade | Nostalgic Steady |

---

## 6. Project Directory Structure

```
dhun/
├── Cargo.toml                       # Cargo workspace configuration
├── package.json                     # Frontend & Tauri dependencies
├── tsconfig.json                    # Strict TypeScript configuration
├── vite.config.ts                   # Vite bundler configuration
├── tailwind.config.js               # Dark DJ console design tokens
│
├── src-tauri/                       # Native Rust Backend & Audio Engine
│   ├── Cargo.toml                   # Rust crates (cpal, symphonia, rusqlite, axum)
│   ├── tauri.conf.json              # Tauri 2.0 window and security configuration
│   ├── src/
│   │   ├── main.rs                  # Entry point, DB migration runner, Tauri builder
│   │   ├── lib.rs                   # Public crate modules for unit testing
│   │   ├── audio/                   # Real-time CPAL audio engine
│   │   │   ├── mod.rs               # Engine loop, command receiver, telemetry emitter
│   │   │   ├── deck.rs              # Virtual Deck state machine (play/pause/cue/scratch)
│   │   │   ├── decoder.rs           # Symphonia-based audio decoding into PCM pools
│   │   │   ├── clock.rs             # Sample-accurate audio clock
│   │   │   ├── device.rs            # Soundcard enumeration and format selection
│   │   │   ├── telemetry.rs         # Peak & RMS VU metering in dBFS
│   │   │   └── dsp/                 # Deterministic DSP graph
│   │   │       ├── biquad.rs        # Direct Form II Transposed biquad filter
│   │   │       ├── eq3.rs           # 3-band Isolator EQ (Low/Mid/High) with kills
│   │   │       ├── filter.rs        # Resonant HPF/LPF sweep
│   │   │       ├── crossfader.rs    # Linear, Equal-Power & Bass-Swap curves
│   │   │       └── limiter.rs       # 1ms lookahead brickwall peak limiter
│   │   ├── db/                      # SQLite persistence layer
│   │   │   ├── mod.rs               # Thread-safe database connection pool
│   │   │   ├── schema.sql           # DDL for all 24 normalized entities
│   │   │   ├── migrations.rs        # Versioned migration engine with seed data
│   │   │   └── models.rs            # Strongly-typed Rust models
│   │   ├── analysis/                # Deterministic signal processing
│   │   │   ├── mod.rs               # Full analysis orchestrator
│   │   │   ├── bpm_detector.rs      # Energy envelope autocorrelation BPM
│   │   │   ├── key_detector.rs      # Chromagram pitch profiling & Camelot wheel
│   │   │   ├── beat_grid.rs         # Downbeat tracking & phase grid generator
│   │   │   └── phrase_detector.rs   # 8/16/32-bar phrase boundary segmentation
│   │   ├── dj/                      # Automatic DJ selection & transition logic
│   │   │   ├── levels.rs            # DJ level capabilities (1 to 4)
│   │   │   ├── moods.rs             # Mood preset definitions & genre weights
│   │   │   ├── selector.rs          # Multi-criteria deterministic scoring
│   │   │   ├── transition.rs        # Transition planner & bass swap scheduler
│   │   │   └── scheduler.rs         # Queue manager & preloading coordinator
│   │   ├── network/                 # Local guest server
│   │   │   ├── mod.rs               # Axum server lifecycle manager
│   │   │   ├── ip.rs                # Local Wi-Fi IP auto-detection
│   │   │   ├── qr.rs                # SVG QR code generator
│   │   │   ├── routes.rs            # Guest REST endpoints & mobile SPA server
│   │   │   └── rate_limit.rs        # Token bucket abuse prevention
│   │   ├── library/                 # Music library & cache
│   │   │   ├── scanner.rs           # Recursive filesystem crawler & metadata
│   │   │   └── cache.rs             # 10-track rolling cache & deck locks
│   │   ├── recovery/                # Diagnostics supervisor & watchdog
│   │   └── commands/                # Tauri IPC bridge handlers
│   └── tests/
│       └── engine_tests.rs          # Automated Unit, DSP, DB & Integration tests
│
└── src/                             # Desktop UI (React 18 + TypeScript)
    ├── App.tsx                      # Root application layout & screen router
    ├── types.ts                     # TypeScript domain interfaces
    ├── api/ipc.ts                   # Tauri IPC & simulated browser fallback bridge
    ├── store/useDhunStore.ts        # Zustand global state store
    └── components/
        ├── layout/Navbar.tsx        # Top status bar, master volume & emergency fade
        └── screens/
            ├── HomeScreen.tsx       # Landing dashboard & party readiness stats
            ├── WizardScreen.tsx     # 6-step party setup wizard
            ├── LivePartyScreen.tsx  # Live dashboard, Now Playing, Next, QR code
            ├── MixerScreen.tsx      # 2/4-deck hardware console, jog wheels, waveforms
            ├── LibraryScreen.tsx    # Music library, search, and track inspector
            ├── RequestsScreen.tsx   # Guest song request moderation queue
            ├── HistoryScreen.tsx    # Past party session logs & analytics
            ├── SettingsScreen.tsx   # Hardware soundcards & cache policies
            └── DiagnosticsScreen.tsx# Real-time audio underruns, CPU load, and watchdog
```

---

## 7. Build and Run Instructions

### Prerequisites
- **Node.js**: v18+ or v20+
- **Rust & Cargo**: Stable toolchain (`x86_64-pc-windows-msvc` on Windows)
- **C++ Build Tools**: Visual Studio 2022 C++ Build Tools installed

### 1. Install Frontend Dependencies
```powershell
cd c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun
npm install
```

### 2. Run Desktop Application in Development Mode
```powershell
# Launches Vite frontend connected to Tauri native Rust backend
npm run tauri dev
```

### 3. Run Standalone Frontend Preview in Browser (with Mock IPC Engine)
```powershell
# Launches Vite dev server on http://localhost:1420
npm run dev
```

### 4. Run Automated Backend Test Suite
```powershell
cd src-tauri
cargo test
```

### 5. Build Release Desktop Installer
```powershell
npm run tauri build
# Outputs native Windows installer (.msi / .exe) in src-tauri/target/release/bundle/
```

---

## 8. Guest Mobile Access Instructions

1. Start the party from the Host desktop app.
2. The laptop automatically binds to the local Wi-Fi IP (e.g. `http://192.168.1.100:3000/`) and renders a QR code.
3. Guests connect their smartphones to the same local Wi-Fi network and scan the QR code using their camera.
4. The mobile web page opens instantly in Safari/Chrome without installing an app.
5. Guests search the laptop's music library and submit requests.
6. Requests appear on the host's screen for instant approval or Force Next override.

