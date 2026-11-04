//! Drum grooves sampled from real drummers (src/compose/groove.rs, groove_model.txt): the model reads, a seed is
//! deterministic and seeds differ, the first bar keeps to the style's band, every generated module renders, the
//! rendered hits are exactly the groove's table, and the lens book now plays generated drums only.

use luminesce::compose::book::{self, book, solo_song, Slot};
use luminesce::compose::groove::{self, groove, syncopation, INST};
use luminesce::render::{render_traced, VoiceTrace};
use luminesce::song::Song;

fn citypop() -> &'static groove::Style {
    groove::style(book::GROOVE_STYLE).expect("the lens book's style is in the model")
}

#[test]
fn the_model_reads_every_style_with_every_instrument() {
    let s = groove::styles();
    assert!(s.len() >= 6, "{} styles", s.len());
    for st in s {
        for k in INST {
            let i = &st.beat.inst[&k];
            assert!(i.p.iter().flatten().all(|p| (0.0..=1.0).contains(p)), "{} {k}: a probability outside 0..1", st.name);
        }
    }
}

#[test]
fn a_bad_model_line_is_refused_with_its_line_number() {
    let e = groove::parse("style x\npart beat\nbogus 1 2 3\n").unwrap_err();
    assert!(e.contains(":3:") && e.contains("bogus"), "{e}");
    let e = groove::parse("style x\npart beat\ninst K dens 1 2 3 change 0.1 hits 5\np0 K 0.5 0.5\n").unwrap_err();
    assert!(e.contains("16 numbers"), "{e}");
    assert!(groove::parse("p0 K 1\n").is_err(), "a line before any style");
}

#[test]
fn the_same_seed_gives_the_same_groove_and_seeds_differ() {
    let a = groove(citypop(), 7);
    let b = groove(citypop(), 7);
    assert_eq!(a.bars, b.bars);
    let distinct: std::collections::BTreeSet<Vec<u8>> = (0..24).map(|s| groove(citypop(), s).bars[0][&'K'].to_vec()).collect();
    assert!(distinct.len() >= 12, "24 seeds gave only {} different kick bars", distinct.len());
}

#[test]
fn the_first_bar_keeps_to_the_styles_band_of_real_bars() {
    let st = citypop();
    let mut inside = 0;
    for seed in 0..50 {
        let g = groove(st, seed);
        let bar = &g.bars[0];
        let mut on = [false; 16];
        for s in 0..16 {
            on[s] = bar[&'K'][s] > 0 || bar[&'S'][s] > 0;
        }
        let sync = syncopation(&on);
        let dens_ok = ['K', 'S', g.keep, 'O', 'T'].iter().all(|k| {
            let n = bar[k].iter().filter(|v| **v > 0).count() as f64;
            let d = st.beat.inst[k].dens;
            n >= d.0 && n <= d.2
        });
        if dens_ok && sync >= st.beat.sync.0 && sync <= st.beat.sync.1 {
            inside += 1;
        }
    }
    assert!(inside >= 48, "only {inside} of 50 first bars inside the band");
}

#[test]
fn the_lens_book_plays_generated_drums_only() {
    for s in [Slot::Kick, Slot::Hat, Slot::Clap, Slot::Perc] {
        let ms: Vec<_> = book().slot(s).collect();
        assert!(ms.len() >= 12, "{}: {} modules", s.name(), ms.len());
        assert!(ms.iter().all(|m| m.id.contains(book::GROOVE_STYLE)), "{}: a written module is still here", s.name());
    }
}

#[test]
fn every_generated_hit_is_the_tables_hit() {
    // Render each generated kick, clap and hat over a progression and compare every drum hit with the groove's table:
    // the step it lands nearest, and its loudness (the digit / 9). Two passes of the 8-bar groove.
    let prog = book().slot(Slot::Progression).next().expect("a progression");
    for seed in [0u64, 5, 11] {
        let g = groove(citypop(), seed);
        for (slot, k) in [(Slot::Kick, 'K'), (Slot::Clap, 'S'), (Slot::Hat, g.keep)] {
            let id = format!("{}.{}{}", slot.name(), g.style, seed);
            let m = book().get(&id).unwrap_or_else(|| panic!("no module {id}"));
            let text = solo_song(m, prog, 24_000);
            let song = Song::parse(&text).unwrap_or_else(|e| panic!("{id}: {e}"));
            let (_, trace) = render_traced(&song, 256).unwrap_or_else(|e| panic!("{id}: {e}"));
            let hits: Vec<(u64, f64)> = trace
                .voices
                .iter()
                .find_map(|v| if let VoiceTrace::Drum(h) = v { Some(h.iter().map(|x| (x.sample, x.amp)).collect()) } else { None })
                .expect("the drum's trace");
            let sixteenth = 24_000.0 * 60.0 / song.bpm / 4.0;
            let mut want = Vec::new();
            for step in 0..(song.beats as usize * 4) {
                let v = g.bars[(step / 16) % 8][&k][step % 16];
                let d = if v == 0 { 0 } else { ((v as f64 / 127.0 * 9.0).round() as u8).clamp(1, 9) };
                if d > 0 {
                    want.push((step, d));
                }
            }
            let got: Vec<(usize, u8)> = hits.iter().map(|(s, a)| (((*s as f64) / sixteenth).round() as usize, (a * 9.0).round() as u8)).collect();
            assert_eq!(got, want, "{id}: the rendered hits are not the table's");
        }
    }
}

#[test]
fn a_slots_grooves_are_one_family_and_written_modules_are_their_own() {
    assert_eq!(groove::family("kick.citypop7"), "kick.citypop");
    assert_eq!(groove::family("kick.citypop12"), groove::family("kick.citypop0"));
    assert_ne!(groove::family("kick.citypop7"), groove::family("hat.citypop7"));
    assert_eq!(groove::family("lead.blancmange"), "lead.blancmange");
    assert_eq!(groove::family("kick.citypop"), "kick.citypop", "no seed: not a generated groove, kept as written");
    assert_eq!(groove::family("prog.royal_road"), "prog.royal_road");
}
