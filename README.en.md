# axon

<div align="center">

[한국어](README.md) &nbsp;|&nbsp; **English**

[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-1.85+-CE422B?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-AGPL--3.0-blue)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey)

A **local-first, deterministic financial & audit analysis desktop for accounting professionals**

</div>

---

Analyze Korean public-disclosure (OpenDART) statements and ledger data **locally** — as tables, charts, and ratios. Every figure, total, and check is computed by deterministic code: zero LLM, fully offline, and your data never leaves the machine. An open analyzer for accounting professionals, filling the gap between expensive proprietary audit tools and developer-only libraries. It's built by a practicing accountant for their own day-to-day work, so features follow what the field actually needs.

> **In development** — OpenDART lookup, multi-year analysis, export, and trend charts work today. Financial ratios, number tie-out, and ledger (audit) analysis are on the roadmap.

## Features

- **Disclosure lookup** — fetch listed-company statements via OpenDART with one company-name search
- **Multi-year range** — start/end years merged into an account × year table (concurrent fetches)
- **Trend charts** — click an account to compare year-by-year trends; index (base = 100) mode compares accounts of very different magnitudes on one plane
- **Export** — Excel (`.xlsx`), CSV, JSON, PDF print
- **Determinism first** — every figure and total in pure Rust, accurate without network or LLM
- **Local-first = confidential** — all deterministic analysis stays on your machine; no audit or client data leaks
- **Dark / Light · KO / EN** — high-contrast themes and a language toggle

Optionally connect a usix daemon for AI audit assistance (working-paper and report drafting, etc.) — this is the only paid part, and without it no data ever leaves your device.

## Getting started

**Requirements** — Rust 1.85+ · Node.js + npm · (Linux) `webkit2gtk-4.1`

```bash
npm install
npm run tauri dev      # run in dev mode
npm run tauri build    # build the desktop installer
```

### OpenDART key

Statement lookup needs an OpenDART API key (free). To issue one:

1. Go to [OpenDART](https://opendart.fss.or.kr) (English site: [engopendart.fss.or.kr](https://engopendart.fss.or.kr)) and open **인증키 신청/관리 (API Key Application) → 인증키 신청 (Apply)**
2. Enter your name, email, and purpose; accept the terms and submit
3. Confirm via the verification email — issued instantly for individuals; companies after a business-registration review
4. Find your key under **인증키 신청/관리 → 오픈API 이용현황 (API Usage Status)**
5. Enter it in the app under **Settings → OpenDART key** — it is stored safely in the OS keychain

> One key allows up to ~20,000 requests per day by default.

## Stack

Tauri 2 · Rust · Vanilla TypeScript + Vite · rust_decimal · rust_xlsxwriter

## License

[AGPL-3.0](LICENSE)
