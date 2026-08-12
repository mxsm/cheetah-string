use cheetah_string::CheetahString;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;
use std::sync::Arc;

struct CountingAllocator;

std::thread_local! {
    // Test-harness bookkeeping may allocate on a different thread while this
    // binary is running. Count only the thread executing the measured closure.
    static TRACKING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATION_COUNT: Cell<usize> = const { Cell::new(0) };
    static ALLOCATED_BYTES: Cell<usize> = const { Cell::new(0) };
}

fn record_allocation(size: usize) {
    let _ = TRACKING.try_with(|tracking| {
        if tracking.get() {
            let _ = ALLOCATION_COUNT.try_with(|count| count.set(count.get() + 1));
            let _ = ALLOCATED_BYTES.try_with(|bytes| bytes.set(bytes.get() + size));
        }
    });
}

// SAFETY: Every operation delegates to the process System allocator.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation(layout.size());
        // SAFETY: The caller provided a valid allocation layout.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: ptr/layout originated from System.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record_allocation(new_size);
        // SAFETY: ptr/layout originated from System and new_size is forwarded.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn measure<T>(operation: impl FnOnce() -> T) -> (usize, usize, T) {
    TRACKING.with(|tracking| tracking.set(false));
    ALLOCATION_COUNT.with(|count| count.set(0));
    ALLOCATED_BYTES.with(|bytes| bytes.set(0));
    TRACKING.with(|tracking| tracking.set(true));
    let result = black_box(operation());
    TRACKING.with(|tracking| tracking.set(false));
    (
        ALLOCATION_COUNT.with(Cell::get),
        ALLOCATED_BYTES.with(Cell::get),
        result,
    )
}

fn assert_clone_allocations(value: &CheetahString) {
    let pointer = value.as_bytes().as_ptr();
    let (count, bytes, cloned) = measure(|| black_box(value).clone());
    assert_eq!(
        (count, bytes),
        (0, 0),
        "clone must only adjust ownership metadata"
    );
    assert_eq!(cloned.as_bytes().as_ptr(), pointer);
    assert_eq!(&cloned, value);
}

fn exact_string(len: usize) -> String {
    let mut value = String::with_capacity(len);
    value.extend(std::iter::repeat_n('x', len));
    assert_eq!(value.len(), value.capacity());
    value
}

fn spare_string(len: usize) -> String {
    let mut value = String::with_capacity(len * 2);
    value.extend(std::iter::repeat_n('x', len));
    assert!(value.capacity() > value.len());
    value
}

#[test]
fn arc_str_char_and_short_concat_are_zero_allocation_fast_paths() {
    let source: Arc<str> = Arc::from("a".repeat(1024));
    let source_pointer = source.as_ptr();
    let (count, bytes, shared) = measure(|| CheetahString::from(black_box(source)));
    assert_eq!((count, bytes), (0, 0));
    assert_eq!(shared.as_bytes().as_ptr(), source_pointer);

    let short_source: Arc<str> = Arc::from("short");
    let short_pointer = short_source.as_ptr();
    let (count, bytes, short) = measure(|| CheetahString::from(black_box(short_source)));
    assert_eq!((count, bytes), (0, 0));
    assert_ne!(short.as_bytes().as_ptr(), short_pointer);

    let (count, bytes, ()) = measure(|| {
        for scalar in 0..=char::MAX as u32 {
            if let Some(character) = char::from_u32(scalar) {
                let value = CheetahString::from(black_box(character));
                let mut encoded = [0; 4];
                assert_eq!(value.as_str(), character.encode_utf8(&mut encoded));
                black_box(value);
            }
        }
    });
    assert_eq!((count, bytes), (0, 0), "every Unicode scalar is inline");

    let left = CheetahString::from("inline");
    let (count, bytes, combined) = measure(|| black_box(left) + black_box("-value"));
    assert_eq!((count, bytes), (0, 0));
    assert_eq!(combined, "inline-value");

    let left = CheetahString::from("inline");
    let owned_right = String::from("-owned");
    let (count, bytes, combined) = measure(|| black_box(left) + black_box(owned_right));
    assert_eq!((count, bytes), (0, 0));
    assert_eq!(combined, "inline-owned");
}

#[test]
fn exact_and_spare_capacity_freeze_costs_are_explicit() {
    let exact_input = exact_string(1024);
    let (count, _, exact) = measure(|| CheetahString::from_string(exact_input));
    assert_eq!(count, 1, "exact-capacity String freeze creates Arc backing");
    assert_eq!(exact.len(), 1024);

    let spare_input = spare_string(1024);
    let (count, _, spare) = measure(|| CheetahString::from_string(spare_input));
    assert_eq!(
        count, 2,
        "spare-capacity String freeze shrinks then creates Arc backing"
    );
    assert_eq!(spare.len(), 1024);

    let mut exact_builder = cheetah_string::CheetahBuilder::with_capacity(1024);
    exact_builder.push_str(&"x".repeat(1024));
    assert_eq!(exact_builder.len(), exact_builder.capacity());
    let (count, _, exact) = measure(|| exact_builder.finish());
    assert_eq!(
        count, 1,
        "exact-capacity builder freeze creates Arc backing"
    );
    assert_eq!(exact.len(), 1024);

    let mut spare_builder = cheetah_string::CheetahBuilder::with_capacity(2048);
    spare_builder.push_str(&"x".repeat(1024));
    assert!(spare_builder.capacity() > spare_builder.len());
    let (count, _, spare) = measure(|| spare_builder.finish());
    assert_eq!(
        count, 2,
        "spare-capacity builder freeze shrinks then creates Arc backing"
    );
    assert_eq!(spare.len(), 1024);
}

#[test]
fn v3_allocation_and_clone_contracts() {
    // Keep all measurements in one test. Run this binary with --test-threads=1
    // so unrelated harness allocations cannot contaminate the global counter.
    let (count, bytes, inline) = measure(|| CheetahString::from(black_box("inline")));
    assert_eq!((count, bytes), (0, 0));
    assert_eq!(inline, "inline");
    let (count, bytes, inline_clone) = measure(|| black_box(&inline).clone());
    assert_eq!((count, bytes), (0, 0));
    assert_eq!(inline_clone, inline);

    let (count, bytes, static_value) = measure(|| {
        CheetahString::from_static_str(black_box(
            "a static string longer than the twenty-three-byte inline boundary",
        ))
    });
    assert_eq!((count, bytes), (0, 0));
    assert!(static_value.len() > 23);
    assert_clone_allocations(&static_value);

    let borrowed = "b".repeat(1024);
    let (count, _, borrowed_value) = measure(|| CheetahString::from(black_box(borrowed.as_str())));
    assert_eq!(count, 1, "long borrowed input has one shared allocation");
    assert_eq!(borrowed_value, borrowed);
    assert_clone_allocations(&borrowed_value);

    let mut exact_owned = String::with_capacity(1024);
    exact_owned.extend(std::iter::repeat_n('o', 1024));
    assert_eq!(exact_owned.len(), exact_owned.capacity());
    let (count, bytes, owned_value) =
        measure(|| CheetahString::from_string(black_box(exact_owned)));
    assert_eq!(count, 1, "freezing owned input creates Arc backing");
    assert!(bytes >= owned_value.len());

    let (count, bytes, owned_clone) = measure(|| black_box(&owned_value).clone());
    assert_eq!((count, bytes), (0, 0), "v3 clone only increments Arc");
    assert_eq!(
        owned_clone.as_bytes().as_ptr(),
        owned_value.as_bytes().as_ptr()
    );
    assert_eq!(owned_clone, owned_value);

    let shared_source = "s".repeat(1024);
    let shared_value = CheetahString::from_string_shared(shared_source);
    let (count, bytes, shared_clone) = measure(|| black_box(&shared_value).clone());
    assert_eq!((count, bytes), (0, 0), "Shared clone only increments Arc");
    assert_eq!(
        shared_clone.as_bytes().as_ptr(),
        shared_value.as_bytes().as_ptr()
    );
    assert_eq!(shared_clone, shared_value);

    #[cfg(feature = "bytes")]
    {
        use cheetah_string::CheetahBytes;

        let raw = bytes::Bytes::from_static(b"zero-copy-payload");
        let payload_pointer = raw.as_ptr();
        let (count, allocated, cheetah_bytes) =
            measure(|| CheetahBytes::from(black_box(raw.clone())));
        assert_eq!((count, allocated), (0, 0));
        assert_eq!(cheetah_bytes.as_bytes().as_ptr(), payload_pointer);

        let (count, allocated, roundtrip) =
            measure(|| bytes::Bytes::from(black_box(cheetah_bytes)));
        assert_eq!((count, allocated), (0, 0));
        assert_eq!(roundtrip.as_ptr(), payload_pointer);
    }
}
