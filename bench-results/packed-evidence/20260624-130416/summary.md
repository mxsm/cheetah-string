# Packed Evidence Capture

- Date: 2026-06-24T13:04:16.4234003+08:00
- Repository: cheetah-string
- Output directory: C:\Users\ljbmx\Desktop\mxsm\cheetah-string\bench-results\packed-evidence\20260624-130416

- `cargo test --features experimental-packed`: running
- `cargo test --features experimental-packed`: PASS, see `packed-test.txt`
- `cargo +nightly miri --version`: miri 0.1.0 (9e2abe0c6a 2026-06-16)
- `cargo +nightly miri test --features experimental-packed packed`: running
- `cargo +nightly miri test --features experimental-packed packed`: PASS, see `packed-miri.txt`
- `ASan runtime`: using `C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64`
- `address sanitizer packed tests`: running
- `address sanitizer packed tests`: PASS, see `packed-asan.txt`
- `ASan runtime`: using `C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64`
- `cargo +nightly fuzz run fuzz_packed_from_bytes`: running
- `cargo +nightly fuzz run fuzz_packed_from_bytes`: PASS, see `fuzz-packed-from-bytes.txt`
- `cargo +nightly fuzz run fuzz_packed_push_str`: running
- `cargo +nightly fuzz run fuzz_packed_push_str`: PASS, see `fuzz-packed-push-str.txt`
- `cargo bench --bench packed --features experimental-packed`: running
- `cargo bench --bench packed --features experimental-packed`: PASS, see `packed-bench.txt`

Total failing required gates: 0
