# Measuring tools (dev only)

Python scripts used to measure the reference recordings and our renders for the graph-instrument
round (v4). They need **numpy** and are not part of the engine, which stays std-only Rust. They
read 16- or 32-bit PCM WAV, mixed to mono and normalised to peak 1. Run them on any WAV:

```
python analyze.py song.wav ...            # band energy % (<150, 150-500, 500-2k, 2-6k, >6k Hz),
                                          # spectral peaks per 8192-sample frame within 40 dB
python peakdiag.py song.wav ...           # the same peak count, broken down per band
python partials.py song.wav t0 t1 [fmin fmax floor_db]   # every spectral peak in a window
python harm.py song.wav t0 t1 [fmin fmax]                # one note's harmonics, dB re h1
python sweep.py song.wav onset [n fmin fmax]             # the pitch track of a hit (a dash)
```

These are hand measurements. The peak tracker is a plain local-maximum count, sensitive to window
sidelobes and to how loud the frame's strongest component is, so compare numbers only between
files measured the same way.
