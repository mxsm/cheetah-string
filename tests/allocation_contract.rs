use cheetah_string::CheetahString;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;

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
    exact_owned.extend(std::iter::repeat('o').take(1024));
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
