use cheetah_string::{CheetahBuilder, CheetahString};
use compact_str::CompactString;
use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};
use serde_json::json;
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

struct CountingAllocator;

static TRACK_ALLOCATIONS: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

// SAFETY: All allocation operations delegate to the process System allocator.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACK_ALLOCATIONS.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        // SAFETY: The caller supplied a valid layout and System is the backing allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: ptr/layout came from the backing System allocator.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if TRACK_ALLOCATIONS.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        }
        // SAFETY: ptr/layout came from System and new_size is forwarded unchanged.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const INLINE_CAPACITY: usize = 23;

#[derive(Clone, Eq, Hash, PartialEq)]
enum ArcStrCandidate {
    Inline(CompactString),
    Shared(Arc<str>),
}

impl ArcStrCandidate {
    fn borrowed(value: &str) -> Self {
        if value.len() <= INLINE_CAPACITY {
            Self::Inline(CompactString::from(value))
        } else {
            Self::Shared(Arc::from(value))
        }
    }

    fn owned(value: String) -> Self {
        if value.len() <= INLINE_CAPACITY {
            Self::Inline(CompactString::from(value))
        } else {
            Self::Shared(value.into_boxed_str().into())
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::Inline(value) => value.as_str(),
            Self::Shared(value) => value,
        }
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
enum ArcStringCandidate {
    Inline(CompactString),
    Shared(Arc<String>),
}

impl ArcStringCandidate {
    fn borrowed(value: &str) -> Self {
        if value.len() <= INLINE_CAPACITY {
            Self::Inline(CompactString::from(value))
        } else {
            Self::Shared(Arc::new(value.to_owned()))
        }
    }

    fn owned(value: String) -> Self {
        if value.len() <= INLINE_CAPACITY {
            Self::Inline(CompactString::from(value))
        } else {
            Self::Shared(Arc::new(value))
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::Inline(value) => value.as_str(),
            Self::Shared(value) => value.as_str(),
        }
    }
}

fn allocation_delta<T>(operation: impl FnOnce() -> T) -> (u64, u64, T) {
    TRACK_ALLOCATIONS.store(false, Ordering::SeqCst);
    ALLOCATIONS.store(0, Ordering::SeqCst);
    ALLOCATED_BYTES.store(0, Ordering::SeqCst);
    TRACK_ALLOCATIONS.store(true, Ordering::SeqCst);
    let value = operation();
    TRACK_ALLOCATIONS.store(false, Ordering::SeqCst);
    (
        ALLOCATIONS.load(Ordering::SeqCst),
        ALLOCATED_BYTES.load(Ordering::SeqCst),
        value,
    )
}

#[cfg(target_os = "windows")]
fn current_rss_bytes() -> Option<u64> {
    use std::ffi::c_void;
    use std::mem::size_of_val;

    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn K32GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }

    let mut counters = ProcessMemoryCounters {
        cb: size_of::<ProcessMemoryCounters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };
    // SAFETY: counters is writable for the exact structure size and the pseudo-handle is valid.
    let ok = unsafe {
        K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            size_of_val(&counters) as u32,
        )
    };
    (ok != 0).then_some(counters.working_set_size as u64)
}

#[cfg(target_os = "linux")]
fn current_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let kib = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))?
        .split_whitespace()
        .next()?
        .parse::<u64>()
        .ok()?;
    Some(kib * 1024)
}

#[cfg(target_os = "macos")]
fn current_rss_bytes() -> Option<u64> {
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    let kib = String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()?;
    Some(kib * 1024)
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn current_rss_bytes() -> Option<u64> {
    None
}

fn exact_string(len: usize) -> String {
    let mut value = String::with_capacity(len);
    value.extend(std::iter::repeat('x').take(len));
    debug_assert_eq!(value.len(), value.capacity());
    value
}

fn spare_string(len: usize) -> String {
    let mut value = String::with_capacity(len * 2);
    value.extend(std::iter::repeat('x').take(len));
    debug_assert!(value.capacity() > value.len());
    value
}

fn emit_allocation_evidence() {
    let long = "x".repeat(1024);
    let (arc_str_borrowed_allocs, arc_str_borrowed_bytes, arc_str_borrowed) =
        allocation_delta(|| ArcStrCandidate::borrowed(&long));
    let arc_str_exact_input = exact_string(1024);
    let (arc_str_exact_allocs, arc_str_exact_bytes, arc_str_exact) =
        allocation_delta(|| ArcStrCandidate::owned(arc_str_exact_input));
    let arc_str_spare_input = spare_string(1024);
    let (arc_str_spare_allocs, arc_str_spare_bytes, arc_str_spare) =
        allocation_delta(|| ArcStrCandidate::owned(arc_str_spare_input));
    let (arc_str_clone_allocs, arc_str_clone_bytes, arc_str_clone) =
        allocation_delta(|| arc_str_exact.clone());

    let (arc_string_borrowed_allocs, arc_string_borrowed_bytes, arc_string_borrowed) =
        allocation_delta(|| ArcStringCandidate::borrowed(&long));
    let arc_string_exact_input = exact_string(1024);
    let (arc_string_exact_allocs, arc_string_exact_bytes, arc_string_exact) =
        allocation_delta(|| ArcStringCandidate::owned(arc_string_exact_input));
    let arc_string_spare_input = spare_string(1024);
    let (arc_string_spare_allocs, arc_string_spare_bytes, arc_string_spare) =
        allocation_delta(|| ArcStringCandidate::owned(arc_string_spare_input));
    let (arc_string_clone_allocs, arc_string_clone_bytes, arc_string_clone) =
        allocation_delta(|| arc_string_exact.clone());

    let arc_str_rss_before = current_rss_bytes();
    let retained_arc_str = (0..10_000)
        .map(|index| ArcStrCandidate::owned(format!("RMQ_SYS_TRACE_TOPIC_{index:05}")))
        .collect::<Vec<_>>();
    let arc_str_rss_after = current_rss_bytes();
    drop(retained_arc_str);

    let arc_string_rss_before = current_rss_bytes();
    let retained_arc_string = (0..10_000)
        .map(|index| ArcStringCandidate::owned(format!("RMQ_SYS_TRACE_TOPIC_{index:05}")))
        .collect::<Vec<_>>();
    let arc_string_rss_after = current_rss_bytes();

    let evidence = json!({
        "schema_version": 1,
        "object_sizes": {
            "Inline|Arc<str>": size_of::<ArcStrCandidate>(),
            "Inline|Arc<String>": size_of::<ArcStringCandidate>(),
            "CheetahString": size_of::<CheetahString>()
        },
        "allocations": {
            "Arc<str>": {
                "borrowed": {"count": arc_str_borrowed_allocs, "bytes": arc_str_borrowed_bytes},
                "owned_exact": {"count": arc_str_exact_allocs, "bytes": arc_str_exact_bytes},
                "owned_spare": {"count": arc_str_spare_allocs, "bytes": arc_str_spare_bytes},
                "clone": {"count": arc_str_clone_allocs, "bytes": arc_str_clone_bytes}
            },
            "Arc<String>": {
                "borrowed": {"count": arc_string_borrowed_allocs, "bytes": arc_string_borrowed_bytes},
                "owned_exact": {"count": arc_string_exact_allocs, "bytes": arc_string_exact_bytes},
                "owned_spare": {"count": arc_string_spare_allocs, "bytes": arc_string_spare_bytes},
                "clone": {"count": arc_string_clone_allocs, "bytes": arc_string_clone_bytes}
            }
        },
        "rss": {
            "Arc<str>": {
                "before_bytes": arc_str_rss_before,
                "after_10000_topics_bytes": arc_str_rss_after,
                "delta_bytes": arc_str_rss_before.zip(arc_str_rss_after)
                    .map(|(before, after)| after.saturating_sub(before))
            },
            "Arc<String>": {
                "before_bytes": arc_string_rss_before,
                "after_10000_topics_bytes": arc_string_rss_after,
                "delta_bytes": arc_string_rss_before.zip(arc_string_rss_after)
                    .map(|(before, after)| after.saturating_sub(before))
            }
        }
    });
    println!("SHARED_BACKING_EVIDENCE={evidence}");

    black_box((
        arc_str_borrowed,
        arc_str_exact,
        arc_str_spare,
        arc_str_clone,
        arc_string_borrowed,
        arc_string_exact,
        arc_string_spare,
        arc_string_clone,
        retained_arc_string,
    ));
}

fn bench_construction(c: &mut Criterion) {
    let borrowed = "x".repeat(1024);
    let mut group = c.benchmark_group("shared_backing_construct");
    group.throughput(Throughput::Bytes(borrowed.len() as u64));

    group.bench_function("Arc<str>/borrowed", |b| {
        b.iter(|| ArcStrCandidate::borrowed(black_box(borrowed.as_str())))
    });
    group.bench_function("Arc<String>/borrowed", |b| {
        b.iter(|| ArcStringCandidate::borrowed(black_box(borrowed.as_str())))
    });
    group.bench_function("CheetahString/shared/borrowed", |b| {
        b.iter(|| black_box(CheetahString::from(black_box(borrowed.as_str()))))
    });
    group.bench_function("Arc<str>/owned_exact", |b| {
        b.iter_batched(
            || exact_string(1024),
            |value| ArcStrCandidate::owned(black_box(value)),
            BatchSize::SmallInput,
        )
    });
    group.bench_function("Arc<String>/owned_exact", |b| {
        b.iter_batched(
            || exact_string(1024),
            |value| ArcStringCandidate::owned(black_box(value)),
            BatchSize::SmallInput,
        )
    });
    group.bench_function("CheetahString/shared/owned_exact", |b| {
        b.iter_batched(
            || exact_string(1024),
            |value| CheetahString::from_string_shared(black_box(value)),
            BatchSize::SmallInput,
        )
    });
    group.bench_function("Arc<str>/owned_spare", |b| {
        b.iter_batched(
            || spare_string(1024),
            |value| ArcStrCandidate::owned(black_box(value)),
            BatchSize::SmallInput,
        )
    });
    group.bench_function("Arc<String>/owned_spare", |b| {
        b.iter_batched(
            || spare_string(1024),
            |value| ArcStringCandidate::owned(black_box(value)),
            BatchSize::SmallInput,
        )
    });
    group.bench_function("CheetahString/shared/owned_spare", |b| {
        b.iter_batched(
            || spare_string(1024),
            |value| CheetahString::from_string_shared(black_box(value)),
            BatchSize::SmallInput,
        )
    });
    group.bench_function("Arc<str>/builder_finish", |b| {
        b.iter_batched(
            || {
                let mut builder = CheetahBuilder::with_capacity(1024);
                builder.push_str(&borrowed);
                builder
            },
            |builder| ArcStrCandidate::owned(black_box(builder.into_string())),
            BatchSize::SmallInput,
        )
    });
    group.bench_function("Arc<String>/builder_finish", |b| {
        b.iter_batched(
            || {
                let mut builder = CheetahBuilder::with_capacity(1024);
                builder.push_str(&borrowed);
                builder
            },
            |builder| ArcStringCandidate::owned(black_box(builder.into_string())),
            BatchSize::SmallInput,
        )
    });
    group.bench_function("CheetahString/shared/builder_finish", |b| {
        b.iter_batched(
            || {
                let mut builder = CheetahBuilder::with_capacity(1024);
                builder.push_str(&borrowed);
                builder
            },
            |builder| CheetahString::from_string_shared(black_box(builder.into_string())),
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn bench_clone(c: &mut Criterion) {
    let value = exact_string(1024);
    let arc_str = ArcStrCandidate::owned(value.clone());
    let arc_string = ArcStringCandidate::owned(value.clone());
    let current_shared = CheetahString::from_string_shared(value);

    let mut group = c.benchmark_group("shared_backing_clone_1kb");
    group.bench_function("Arc<str>", |b| {
        b.iter(|| black_box(black_box(&arc_str).clone()))
    });
    group.bench_function("Arc<String>", |b| {
        b.iter(|| black_box(black_box(&arc_string).clone()))
    });
    group.bench_function("CheetahString/shared", |b| {
        b.iter(|| black_box(black_box(&current_shared).clone()))
    });
    group.finish();
}

fn properties() -> Vec<(&'static str, &'static str)> {
    vec![
        ("KEYS", "order-10001"),
        ("TAGS", "paid"),
        ("WAIT", "false"),
        ("RETRY_TOPIC", "order-service"),
        ("REAL_TOPIC", "order-created"),
        ("PGROUP", "order-consumer"),
        ("UNIQ_KEY", "7F00000100002A9F000000000001"),
    ]
}

fn bench_mq_workloads(c: &mut Criterion) {
    let property_entries = properties();
    let headers = properties();
    let topics = (0..10_000)
        .map(|index| format!("RMQ_SYS_TRACE_TOPIC_{index:05}"))
        .collect::<Vec<_>>();
    let mut group = c.benchmark_group("shared_backing_mq");

    for candidate in ["Arc<str>", "Arc<String>", "CheetahString/shared"] {
        group.bench_with_input(
            BenchmarkId::new("property_build", candidate),
            &candidate,
            |b, candidate| {
                b.iter_batched(
                    || property_entries.clone(),
                    |entries| {
                        if *candidate == "Arc<str>" {
                            let map = entries
                                .into_iter()
                                .map(|(key, value)| {
                                    (
                                        ArcStrCandidate::borrowed(black_box(key)),
                                        ArcStrCandidate::borrowed(black_box(value)),
                                    )
                                })
                                .collect::<HashMap<_, _>>();
                            black_box(map);
                        } else if *candidate == "Arc<String>" {
                            let map = entries
                                .into_iter()
                                .map(|(key, value)| {
                                    (
                                        ArcStringCandidate::borrowed(black_box(key)),
                                        ArcStringCandidate::borrowed(black_box(value)),
                                    )
                                })
                                .collect::<HashMap<_, _>>();
                            black_box(map);
                        } else {
                            let map = entries
                                .into_iter()
                                .map(|(key, value)| {
                                    (
                                        CheetahString::from(black_box(key)),
                                        CheetahString::from(black_box(value)),
                                    )
                                })
                                .collect::<HashMap<_, _>>();
                            black_box(map);
                        }
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("header_encode", candidate),
            &candidate,
            |b, candidate| {
                b.iter(|| {
                    let mut encoded = String::with_capacity(256);
                    for (key, value) in black_box(&headers) {
                        if *candidate == "Arc<str>" {
                            encoded.push_str(ArcStrCandidate::borrowed(key).as_str());
                            encoded.push('=');
                            encoded.push_str(ArcStrCandidate::borrowed(value).as_str());
                        } else if *candidate == "Arc<String>" {
                            encoded.push_str(ArcStringCandidate::borrowed(key).as_str());
                            encoded.push('=');
                            encoded.push_str(ArcStringCandidate::borrowed(value).as_str());
                        } else {
                            encoded.push_str(CheetahString::from(*key).as_str());
                            encoded.push('=');
                            encoded.push_str(CheetahString::from(*value).as_str());
                        }
                        encoded.push('\n');
                    }
                    black_box(encoded)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("topic_insert", candidate),
            &candidate,
            |b, candidate| {
                b.iter_batched(
                    || (),
                    |_| {
                        if *candidate == "Arc<str>" {
                            black_box(
                                topics
                                    .iter()
                                    .enumerate()
                                    .map(|(index, topic)| {
                                        (ArcStrCandidate::borrowed(black_box(topic)), index)
                                    })
                                    .collect::<HashMap<_, _>>(),
                            );
                        } else if *candidate == "Arc<String>" {
                            black_box(
                                topics
                                    .iter()
                                    .enumerate()
                                    .map(|(index, topic)| {
                                        (ArcStringCandidate::borrowed(black_box(topic)), index)
                                    })
                                    .collect::<HashMap<_, _>>(),
                            );
                        } else {
                            black_box(
                                topics
                                    .iter()
                                    .enumerate()
                                    .map(|(index, topic)| {
                                        (CheetahString::from(black_box(topic.as_str())), index)
                                    })
                                    .collect::<HashMap<_, _>>(),
                            );
                        }
                    },
                    BatchSize::LargeInput,
                )
            },
        );
    }
    group.finish();
}

fn benchmarks(c: &mut Criterion) {
    emit_allocation_evidence();
    bench_construction(c);
    bench_clone(c);
    bench_mq_workloads(c);
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
