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

---

## Iterasi strategi #1 dan #2 — `trend_regime_strategy`

Setelah audit di atas, dibuat strategi aktif kedua, `trend_regime_strategy`
(`src/strategy/trend_regime.rs`), untuk menguji langsung permintaan pemilik
project: klasifikasi regime dulu (pakai `classify_basic_regime` yang sudah
ada, dari screening timeframe), baru evaluasi entry. Diuji berdampingan
dengan `basic_sample_strategy` lewat `strategy_run_mode = "comparison"`
(`config/research_strategy_comparison.toml`), data dan risk sizing identik
(`risk_per_trade_pct = 1.0`, `max_drawdown_pct = 20.0`).

**Aturan `trend_regime_strategy`**: skip total kalau regime screening-timeframe
`Ranging`/`Unknown`. Kalau `Bullish`/`Bearish`, perlu entry-timeframe
EMA-8/EMA-21 dan posisi close vs VWAP setuju arah regime tsb.

### Versi 1 — stop 1x ATR, TP 2x ATR (RR 2.0)

| Metrik | basic_sample_strategy | trend_regime_strategy v1 |
|---|---|---|
| total_trades | 68 | 70 |
| win_rate | 47.06% | 38.57% |
| profit_factor | 0.499 | 0.514 |
| expectancy/trade | -14.04 | -14.45 |
| avg_expected_edge_bps | 22.04 | 29.74 |
| avg_actual_edge_bps | -10.58 | -10.54 |
| avg_edge_realization_bps | -32.62 | -40.28 |

Hasil: **hampir identik dengan basic_sample**, regime-gating + RR lebih lebar
tidak mengubah tanda expectancy.

### Versi 2 — stop 2x ATR, TP 4x ATR (RR tetap 2.0, jarak diperlebar)

Hipotesis diuji: apakah stop 1x ATR terlalu sempit untuk noise candle 1-menit
BTCUSDT, sehingga sering kena SL sebelum tren beneran jalan?

| Metrik | trend_regime_strategy v1 (stop sempit) | trend_regime_strategy v2 (stop lebar) |
|---|---|---|
| total_trades | 70 | 71 |
| win_rate | 38.57% | 35.21% |
| profit_factor | 0.514 | 0.410 |
| expectancy/trade | -14.45 | -14.24 |
| avg_edge_realization_bps | -40.28 | **-43.25 (lebih buruk)** |

**Hipotesis ini terbukti salah** — memperlebar stop tidak memperbaiki apapun,
malah realisasi edge sedikit lebih buruk. Kesimpulan: bukan soal stop
kesempitan.

### Temuan teknis konkret yang relevan untuk iterasi berikutnya

`stop_slippage_bps = 5.0` di `[cost]` hanya dibebankan pada exit via
stop-loss (`src/backtest/engine.rs:608`), sementara `expected_reward_bps`/
`expected_net_edge_bps` yang dihitung strategi memakai `estimated_cost_bps`
flat, tidak membedakan skenario menang/kalah. Dengan win rate di bawah 50%
untuk kedua strategi yang diuji, biaya ekstra ini secara sistematis lebih
sering kena di trade yang rugi — salah satu kontributor konkret ke gap
avg_edge_realization_bps yang selalu negatif besar (-33 sampai -43 bps) di
ketiga varian yang sudah diuji (basic_sample, trend_regime v1, trend_regime
v2).

### Kesimpulan sementara setelah 3 varian diuji

Tiga strategi/varian berbeda (basic_sample, trend_regime v1, trend_regime v2)
semuanya menunjukkan pola yang sama: expectancy negatif, profit_factor di
bawah 1, dan gap besar antara edge yang "diharapkan" vs yang "terealisasi".
Ini indikasi bahwa masalahnya bukan di pemilihan arah (long/short) atau lebar
stop, tapi kemungkinan di salah satu dari:

1. Frekuensi entry yang terlalu tinggi relatif terhadap horizon 1-menit,
   sehingga terlalu banyak entry di titik yang secara statistik tidak punya
   edge (konsisten dengan temuan riset ML sebelumnya bahwa sinyal di
   horizon pendek sangat lemah setelah cost).
2. Cost asimetris (stop_slippage_bps hanya kena saat rugi) yang belum
   dimasukkan ke perhitungan "expected edge" strategi.
3. ATR(14) di 1-menit terlalu reaktif terhadap noise jangka pendek untuk
   dijadikan basis stop/TP yang stabil.

**Belum ada strategi dengan expectancy positif ditemukan.** Iterasi
selanjutnya yang lebih masuk akal untuk dicoba: menaikkan entry-timeframe
(mis. dari 1m ke 5m/15m, mengurangi jumlah entry tapi menyaring noise),
bukan terus mengubah stop/TP di timeframe 1-menit yang sama.

---

## Iterasi #3 — cycle "screening → entry → audit posisi → action" (real, bukan cuma config)

Pemilik project secara eksplisit meminta: entry-timeframe minimal 5 menit,
DAN jangan asal generate sinyal tiap bar, tapi ada siklus screening → entry
→ **audit posisi** (selama posisi terbuka, terus dicek apakah masih valid)
→ action (hold/close).

**Yang sudah otomatis benar tanpa perubahan kode**: engine
(`src/backtest/engine.rs`) memang sudah tidak pernah mengevaluasi sinyal
baru selama ada posisi terbuka (`open_position.is_none()` adalah syarat
sebelum `strategy.evaluate()` dipanggil) — jadi "jangan entry tiap bar" itu
sudah otomatis benar begitu `max_open_positions = 1`.

**Yang genuinely belum ada dan baru dibangun sekarang**: "audit posisi" —
sebelumnya, begitu posisi terbuka, satu-satunya cara keluar adalah level
statis (SL/TP) atau time-exit; tidak ada pengecekan aktif ulang tiap bar.
Ditambahkan:

- `Strategy::audit_position(ctx, input, open_side) -> PositionAction`
  (`src/strategy/traits.rs`) — dipanggil oleh engine sekali per bar
  entry-timeframe selama posisi terbuka, SETELAH pengecekan SL/TP/time-exit
  statis tidak menemukan exit di bar itu. Default: selalu `Hold` (strategi
  lama tidak berubah perilakunya kalau tidak override method ini).
- `PositionAction::{Hold, CloseNow { reason }}` — kalau `CloseNow`, posisi
  ditutup saat itu juga di harga close bar tsb, dicatat sebagai
  `TradeExitReason::ManualClose` (`FillModel::strategy_close_exit`,
  `src/backtest/fill_model.rs`).
- `trend_regime_strategy` sekarang override `audit_position`: tiap bar,
  cek ulang regime screening-timeframe. Kalau regime yang jadi alasan masuk
  sudah berbalik (atau melemah jadi Ranging/Unknown), tutup posisi sekarang
  — tidak menunggu SL/TP kena.

Ini genuinely capability baru di execution engine, bukan sekadar ganti
config, dan sudah diuji (`cargo test`, 447 test lulus, termasuk 3 test baru
untuk `audit_position`).

### Hasil dengan entry_timeframe = 5m, confirmation = 15m, screening = 1h

(`config/research_5m_entry_audit.toml`, risk sizing sama:
`risk_per_trade_pct = 1.0`, `max_drawdown_pct = 20.0`)

| Metrik | basic_sample (5m entry) | trend_regime (5m entry + audit) |
|---|---|---|
| total_trades | 56 | 68 |
| win_rate | 42.86% | 35.29% |
| profit_factor | 0.345 | 0.536 |
| expectancy/trade | **-17.77** (lebih buruk dari versi 1m: -14.04) | -14.96 |
| avg_edge_realization_bps | -34.48 | **-81.04 (jauh lebih buruk dari versi 1m: -40.28)** |

Breakdown exit reason `trend_regime_strategy`: 35 stop_loss, 18 take_profit,
15 time_exit, **0 manual_close**. Artinya mekanisme audit posisi sudah
terpasang dan aktif dicek tiap bar, tapi **regime screening-timeframe (1h)
jarang benar-benar berbalik dalam durasi tahan posisi rata-rata (11-36 bar
5-menit ≈ 1-3 jam)** — jadi belum pernah memicu penutupan dini di dataset ini.

### Kesimpulan jujur setelah 4 varian diuji

Memperlambat entry-timeframe ke 5m **tidak memperbaiki, malah memperburuk**
gap antara edge yang diharapkan vs terealisasi (-81 bps vs -40 bps di 1m).
Basic_sample_strategy juga lebih buruk di 5m (-17.77) dibanding versi 1m-nya
(-14.04). Empat varian yang sudah diuji (basic_sample 1m, trend_regime v1,
trend_regime v2, trend_regime 5m+audit) semuanya menunjukkan expectancy
negatif di kisaran serupa (-14 sampai -18 per trade dari $5.000 modal, risk
1%/trade).

Ini konsisten dengan temuan riset ML forecast jauh sebelumnya
(`docs/forecast-ml-result-analysis.md`): sinyal cost-adjusted di BTCUSDT
1-menit sampai 4-jam sama-sama lemah setelah biaya realistis. Indikasinya
makin kuat bahwa masalahnya bukan di timeframe, lebar stop, atau mekanisme
audit posisi — kemungkinan besar di **logika entry itu sendiri** (trend
alignment EMA/VWAP sederhana) yang belum punya edge nyata di BTCUSDT pada
periode 2020-2025 ini, di timeframe manapun yang sudah dicoba.

**Rekomendasi jujur**: sebelum mencoba parameter lain lagi di keluarga
strategi trend-following yang sama, pertimbangkan untuk menguji jenis logika
entry yang benar-benar berbeda (misalnya mean-reversion, atau breakout
dengan konfirmasi volume, bukan variasi dari trend-alignment EMA/VWAP yang
sudah 4 kali diuji dengan hasil serupa).
