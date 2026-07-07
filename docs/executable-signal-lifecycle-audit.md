# Executable Signal Lifecycle — Audit & Fase 1 Correction

## Ringkasan untuk yang bingung ini bot trading atau bot riset

**Koreksi penting terhadap asumsi sebelumnya**: lifecycle eksekusi (signal → entry →
TP/SL/timeout → trade → ledger) **sudah ada dan sudah terbukti jalan** di kode ini
(`src/backtest/engine.rs`, `src/backtest/fill_model.rs`, `src/backtest/geometry.rs`,
`src/core/trade.rs`). Ini bukan sesuatu yang perlu dibangun dari nol seperti anggapan
di awal diskusi. Yang belum ada, dan memang jadi sumber kebingungan, adalah:

1. Belum pernah ada yang membaca hasil dari lifecycle ini secara jujur dan
   menunjukkannya row-by-row ke pemilik project.
2. Konfigurasi risiko yang dipakai di `config/research.toml` ternyata sangat
   ceroboh (lihat Temuan 1), sehingga hasilnya (kebangkrutan total) tidak
   pernah benar-benar diperiksa maknanya.
3. Strategi yang jalan (`basic_sample_strategy`) diberi label "sample"/
   "reference implementation" di komentar kode, padahal secara isi ini adalah
   strategi teknikal yang nyata dan bisa diaudit — bukan strategi buang-buang.

## Apa itu `basic_sample_strategy`, persisnya

Aturan entry (deterministik, dari `src/strategy/basic_sample.rs`):

- **Long**: harga entry-timeframe di atas VWAP, EMA-8 > EMA-21 (entry timeframe),
  DAN harga confirmation-timeframe di atas VWAP-nya, DAN harga screening-timeframe
  di atas EMA-50-nya (jadi 3 timeframe harus sepakat arah).
- **Short**: kebalikannya, semua kondisi di atas dibalik.
- **Stop-loss**: 1× ATR(14) dari harga entry.
- **Take-profit**: 1.5× ATR(14) dari harga entry (reward:risk 1.5).
- **Time exit**: kalau tidak kena SL/TP dalam `max_bars_held` (18 bar) di
  `config/research.toml`.

Ini strategi trend-alignment multi-timeframe yang wajar dan bisa dievaluasi —
bukan sesuatu yang absurd. Masalahnya bukan di sini; masalahnya ada di dua
temuan di bawah.

## Temuan 1 — Position sizing di config lama tidak masuk akal

`config/research.toml` (versi lama, belum diubah) memakai:

```toml
risk_per_trade_pct = 0.15   # 15% equity dipertaruhkan PER TRADE
max_drawdown_pct = 100.0    # tidak ada circuit breaker sama sekali
```

15% risiko per trade itu jauh di luar standar praktik apapun (umumnya 0.5–2%),
dan `max_drawdown_pct = 100.0` berarti sistem dibiarkan trading sampai modal
benar-benar nol tanpa direm. Hasilnya, dari run yang sudah pernah dijalankan
sebelumnya (`reports/basic_sample_btc_entry1m_2020_2025/`, BTCUSDT 2020–2025,
1 menit):

| Metrik | Nilai |
|---|---|
| total_trades | 24,096 |
| win_rate | 34.41% |
| profit_factor | 0.268 |
| net_pnl | **-5,000.00** (modal awal $5,000, habis total) |
| max_drawdown | 100.00% |
| max_consecutive_losses | 20 |

Equity curve-nya rata di 0.000000 selama sisa periode setelah modal habis —
akun mati di tengah jalan, sisanya cuma noise. **Ini bukti nyata pentingnya
risk management**, tapi belum menjawab apakah strateginya sendiri punya edge.

## Temuan 2 — Setelah sizing diperbaiki, strateginya sendiri tetap rugi

Dibuat `config/research_sane_risk.toml` — **identik** dengan config lama
(strategi sama, data sama, cost model sama), hanya mengubah:

```toml
risk_per_trade_pct = 1.0    # dari 15.0 -> 1.0
max_drawdown_pct = 20.0     # dari 100.0 -> 20.0 (circuit breaker nyata)
```

Hasil (`reports/basic_sample_btc_entry1m_2020_2025_sane_risk/`):

| Metrik | Nilai |
|---|---|
| total_trades | 68 |
| win_rate | 47.06% |
| profit_factor | 0.499 |
| net_pnl | -954.60 |
| expectancy per trade | **-14.04 USD (~-0.28% dari equity)** |
| max_drawdown | 20.10% (circuit breaker bekerja, trading dihentikan) |

Circuit breaker bekerja seperti seharusnya — begitu drawdown 20% tercapai
(setelah 68 trade), `risk/guard.rs` menolak semua sinyal berikutnya
(`max_drawdown_reached`, 933,511 penolakan tercatat sepanjang sisa 6 tahun
data). Akun tidak sampai nol, tapi juga tidak lagi trading.

**Yang penting**: expectancy per trade (~-0.2% sampai -0.28% dari equity) hampir
sama besarnya antara config lama dan config sane-risk. Ini menunjukkan
**negative expectancy adalah sifat dari logika entry/exit itu sendiri, bukan
akibat position sizing**. Position sizing yang benar mencegah kebangkrutan
total, tapi tidak mengubah tanda (+/-) dari edge strategi.

Breakdown tambahan (config sane-risk):

| Exit reason | Jumlah | Win rate |
|---|---|---|
| stop_loss | 36 | 0% (by definition) |
| take_profit | 31 | 100% (by definition) |
| time_exit | 1 | 100% |

36 kena SL vs 31 kena TP — rasio kemenangan/kekalahan hampir seimbang di
jumlah trade, tapi reward:risk 1.5 tidak cukup mengkompensasi cost (rata-rata
fee+slippage per trade cukup besar relatif terhadap posisi kecil ini).

## Jawaban langsung untuk pertanyaan "bot ini jalan pakai apa"

- **Signal**: diproduksi oleh `basic_sample_strategy`, aturan di atas — bukan ML,
  bukan misteri, 100% bisa dibaca di `src/strategy/basic_sample.rs`.
- **Entry/TP/SL**: dari signal itu langsung, disimulasikan oleh
  `backtest::FillModel` dengan model fill konservatif (worst-case dalam bar).
- **Trade ledger**: `trades.csv`, satu baris = satu trade, lengkap dengan
  entry/exit price, waktu, fee, slippage, PnL, alasan keluar. Bisa dibuka dan
  dibaca manual sekarang juga.
- **Live/paper trading**: **belum ada** — `src/execution/mod.rs` dan
  `src/journal/mod.rs` masih placeholder kosong. Semua yang ada sekarang adalah
  simulasi backtest historis, belum terhubung ke exchange manapun.

## Kesimpulan Fase 1

Fase 1 ("Executable Signal Lifecycle Engine") **secara mekanis sudah selesai
sebelum prompt ini ditulis** — infrastrukturnya nyata, teruji, dan sudah
menghasilkan trade ledger yang bisa diaudit habis-habisan. Yang salah bukan
"belum ada lifecycle", tapi:

1. Konfigurasi risk yang dipakai sebelumnya ceroboh — **sudah diperbaiki** di
   `config/research_sane_risk.toml`.
2. Belum pernah ada yang membaca hasilnya secara jujur — **sudah dilakukan**
   di dokumen ini.
3. Strategi yang ada (`basic_sample_strategy`) terbukti **negative expectancy**
   setelah cost realistis, pada horizon backtest 6 tahun BTCUSDT 1-menit ini.

## Rekomendasi Fase 2 (bukan Fase 1 lagi)

Karena lifecycle & audit trail sudah terbukti berfungsi, langkah berikutnya
murni soal **edge strategi**, bukan infrastruktur:

1. **Jangan pakai `basic_sample_strategy` apa adanya untuk uang sungguhan** —
   sudah terbukti negative expectancy di data historis ini.
2. Revisi aturan entry/exit-nya (bukan bikin strategi baru dari nol setiap
   kali) — misalnya uji apakah menghapus salah satu syarat multi-timeframe,
   mengubah reward:risk, atau menambah filter regime mengubah tanda
   expectancy-nya.
3. Tetap pakai `config/research_sane_risk.toml` (atau risk_per_trade_pct serupa,
   1%, drawdown breaker aktif) sebagai baseline wajib untuk setiap revisi
   berikutnya — jangan pernah lagi menjalankan backtest dengan sizing seceroboh
   config lama.
4. ML forecast (`src/forecast/`) tetap dibekukan sampai ada strategi dengan
   expectancy positif (sebelum cost tambahan dari ML) untuk difilter.

## Catatan lingkup

Tidak ada perubahan pada logika strategi, backtest engine, risk guard, atau
model cost. Perubahan hanya: (1) file config baru `config/research_sane_risk.toml`
untuk perbandingan sizing, (2) dokumen ini. `reports/` tetap gitignored dan
tidak di-commit.
