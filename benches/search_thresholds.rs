//! Times the substring-search strategies behind `CheetahString::contains`,
//! `find`, and `rfind`, so the length thresholds in `src/search.rs` can be
//! checked on each target.
//!
//! Prints Markdown tables of median nanoseconds per call. Run it with
//! `cargo bench --bench search_thresholds`; the `Search thresholds` workflow
//! runs it on x86_64 and aarch64.

use std::hint::black_box;
use std::time::{Duration, Instant};

use cheetah_string::CheetahString;

const HAYSTACK_LENS: [usize; 17] = [
    8, 16, 20, 24, 28, 32, 40, 48, 64, 96, 128, 160, 192, 256, 512, 1024, 4096,
];
const NEEDLE_LENS: [usize; 5] = [2, 4, 8, 16, 32];
const TEXT: &[u8] = b"topic-orders-group-consumer-broker-queue-";
const SAMPLES: usize = 15;
const SAMPLE_TIME: Duration = Duration::from_micros(800);
/// A `CheetahString` time this many times the faster strategy is flagged.
const FLAG_RATIO: f64 = 1.15;

#[derive(Clone, Copy)]
enum Op {
    Contains,
    Find,
    Rfind,
}

impl Op {
    fn name(self) -> &'static str {
        match self {
            Op::Contains => "contains",
            Op::Find => "find",
            Op::Rfind => "rfind",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Miss,
    LateHit,
}

impl Case {
    fn name(self) -> &'static str {
        match self {
            Case::Miss => "miss",
            Case::LateHit => "late hit",
        }
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Common,
    Rare,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Common => "common-byte needle",
            Kind::Rare => "rare-byte needle",
        }
    }
}

fn text(len: usize) -> Vec<u8> {
    TEXT.iter().copied().cycle().take(len).collect()
}

/// Returns an `n`-byte needle that never occurs in `base`, the haystack
/// without a late hit. A common-byte needle is cut from `base` and only its
/// last byte is changed, to another byte of `base`, so every needle byte
/// occurs in the haystack and prefilters keep finding candidate matches. A
/// rare-byte needle then swaps its middle byte for one `base` never contains.
fn absent_needle(base: &[u8], n: usize, kind: Kind) -> Vec<u8> {
    let mut needle: Vec<u8> = (0..n).map(|i| base[(7 + i) % base.len()]).collect();
    let mut bytes = base.to_vec();
    bytes.sort_unstable();
    bytes.dedup();
    for &last in &bytes {
        needle[n - 1] = last;
        if memchr::memmem::find(base, &needle).is_none() {
            if let Kind::Rare = kind {
                assert!(
                    !base.contains(&b'Z'),
                    "the haystack text must not contain Z"
                );
                needle[n / 2] = b'Z';
            }
            return needle;
        }
    }
    panic!(
        "no absent {n}-byte needle for a {}-byte haystack",
        base.len()
    );
}

/// Builds the haystack for one cell. A late hit puts the needle where the
/// scan ends: at the end for forward searches and at the start for `rfind`.
fn haystack(base: &[u8], needle: &[u8], op: Op, case: Case) -> String {
    let mut bytes = base.to_vec();
    if case == Case::LateHit {
        let at = match op {
            Op::Contains | Op::Find => bytes.len() - needle.len(),
            Op::Rfind => 0,
        };
        bytes[at..at + needle.len()].copy_from_slice(needle);
        let found = match op {
            Op::Contains | Op::Find => memchr::memmem::find(&bytes, needle),
            Op::Rfind => memchr::memmem::rfind(&bytes, needle),
        };
        assert_eq!(
            found,
            Some(at),
            "the inserted needle must be the match the scan reaches"
        );
    }
    String::from_utf8(bytes).expect("the text is ASCII")
}

fn run<F: FnMut() -> usize>(search: &mut F, iterations: u32) -> f64 {
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(search());
    }
    start.elapsed().as_secs_f64() * 1e9 / f64::from(iterations)
}

fn calibrate<F: FnMut() -> usize>(search: &mut F) -> u32 {
    let mut iterations = 1u32;
    loop {
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(search());
        }
        if start.elapsed() >= SAMPLE_TIME || iterations >= 1 << 24 {
            return iterations;
        }
        iterations *= 2;
    }
}

fn median(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

/// Returns median nanoseconds per call for the three strategies. Samples are
/// taken in turn so drift on a shared runner affects all three alike.
fn measure3<A, B, C>(mut std: A, mut memmem: B, mut cheetah: C) -> [f64; 3]
where
    A: FnMut() -> usize,
    B: FnMut() -> usize,
    C: FnMut() -> usize,
{
    let iterations = [
        calibrate(&mut std),
        calibrate(&mut memmem),
        calibrate(&mut cheetah),
    ];
    let mut samples: [Vec<f64>; 3] = Default::default();
    for _ in 0..SAMPLES {
        samples[0].push(run(&mut std, iterations[0]));
        samples[1].push(run(&mut memmem, iterations[1]));
        samples[2].push(run(&mut cheetah, iterations[2]));
    }
    samples.map(median)
}

fn measure(op: Op, haystack: &str, value: &CheetahString, needle: &str) -> [f64; 3] {
    let found = |index: Option<usize>| index.unwrap_or(usize::MAX);
    match op {
        Op::Contains => measure3(
            || usize::from(black_box(haystack).contains(black_box(needle))),
            || {
                let hit = memchr::memmem::find(
                    black_box(haystack).as_bytes(),
                    black_box(needle).as_bytes(),
                );
                usize::from(hit.is_some())
            },
            || usize::from(black_box(value).contains(black_box(needle))),
        ),
        Op::Find => measure3(
            || found(black_box(haystack).find(black_box(needle))),
            || {
                found(memchr::memmem::find(
                    black_box(haystack).as_bytes(),
                    black_box(needle).as_bytes(),
                ))
            },
            || found(black_box(value).find(black_box(needle))),
        ),
        Op::Rfind => measure3(
            || found(black_box(haystack).rfind(black_box(needle))),
            || {
                found(memchr::memmem::rfind(
                    black_box(haystack).as_bytes(),
                    black_box(needle).as_bytes(),
                ))
            },
            || found(black_box(value).rfind(black_box(needle))),
        ),
    }
}

fn cell([std, memmem, cheetah]: [f64; 3]) -> String {
    let (std, memmem, best) = if std <= memmem {
        (format!("**{std:.1}**"), format!("{memmem:.1}"), std)
    } else {
        (format!("{std:.1}"), format!("**{memmem:.1}**"), memmem)
    };
    let flag = if cheetah > best * FLAG_RATIO {
        " !"
    } else {
        ""
    };
    format!("{std} / {memmem} / {cheetah:.1}{flag}")
}

fn print_table(op: Op, case: Case, kind: Kind) {
    println!();
    println!("## {}, {}, {}", op.name(), case.name(), kind.name());
    println!();
    let header: String = NEEDLE_LENS.iter().map(|n| format!(" n = {n} |")).collect();
    println!("| haystack |{header}");
    println!("| ---: |{}", " --- |".repeat(NEEDLE_LENS.len()));
    for &len in &HAYSTACK_LENS {
        let base = text(len);
        let mut row = format!("| {len} |");
        for &n in &NEEDLE_LENS {
            if n > len {
                row.push_str(" - |");
                continue;
            }
            let needle = String::from_utf8(absent_needle(&base, n, kind)).expect("ASCII");
            let haystack = haystack(&base, needle.as_bytes(), op, case);
            let value = CheetahString::from(haystack.as_str());
            let times = measure(op, &haystack, &value, &needle);
            row.push_str(&format!(" {} |", cell(times)));
        }
        println!("{row}");
    }
}

fn main() {
    println!(
        "# Search strategy timings ({}-{})",
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    println!();
    println!(
        "Median ns per call over {SAMPLES} samples. Each cell is `std / memmem / CheetahString`."
    );
    println!(
        "The faster of std and memmem is bold, and `!` marks a CheetahString time more than {:.0}% above it.",
        (FLAG_RATIO - 1.0) * 100.0
    );
    println!("A miss searches for a needle that is absent from the haystack; a late hit places the needle where the scan ends.");
    println!("A common-byte needle uses only bytes that occur in the haystack; a rare-byte needle contains one byte that does not.");

    for op in [Op::Contains, Op::Find, Op::Rfind] {
        for case in [Case::Miss, Case::LateHit] {
            for kind in [Kind::Common, Kind::Rare] {
                print_table(op, case, kind);
            }
        }
    }
}
