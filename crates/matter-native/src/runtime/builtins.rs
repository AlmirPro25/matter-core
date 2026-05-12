//! Built-in functions for Matter runtime
//!
//! These functions are linked into native executables to provide
//! core functionality like printing, memory allocation, etc.

/// Print an integer to stdout
#[no_mangle]
pub extern "C" fn matter_print_int(value: i64) {
    println!("{}", value);
}

/// Print a boolean to stdout
#[no_mangle]
pub extern "C" fn matter_print_bool(value: bool) {
    println!("{}", value);
}

/// Print a string to stdout
///
/// # Safety
///
/// `ptr` must be valid for reads of `len` bytes for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn matter_print_string(ptr: *const u8, len: usize) {
    if ptr.is_null() {
        return;
    }

    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    if let Ok(s) = std::str::from_utf8(slice) {
        println!("{}", s);
    }
}

/// Allocate memory on the heap
#[no_mangle]
pub extern "C" fn matter_alloc(size: usize) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }

    let layout = match std::alloc::Layout::from_size_align(size, 8) {
        Ok(layout) => layout,
        Err(_) => return std::ptr::null_mut(),
    };
    unsafe { std::alloc::alloc(layout) }
}

/// Free memory allocated by matter_alloc
///
/// # Safety
///
/// `ptr` and `size` must match a previous successful call to `matter_alloc`.
#[no_mangle]
pub unsafe extern "C" fn matter_free(ptr: *mut u8, size: usize) {
    if ptr.is_null() || size == 0 {
        return;
    }

    let layout = match std::alloc::Layout::from_size_align(size, 8) {
        Ok(layout) => layout,
        Err(_) => return,
    };
    unsafe { std::alloc::dealloc(ptr, layout) }
}

/// Panic handler for Matter programs
///
/// # Safety
///
/// `msg_ptr` must be null or valid for reads of `msg_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn matter_panic(msg_ptr: *const u8, msg_len: usize) -> ! {
    if !msg_ptr.is_null() {
        let slice = unsafe { std::slice::from_raw_parts(msg_ptr, msg_len) };
        if let Ok(s) = std::str::from_utf8(slice) {
            eprintln!("Matter panic: {}", s);
        }
    }
    std::process::exit(1);
}

// ============================================================================
// SPRINT 26 PHASE 4: DATA STRUCTURES RUNTIME
// ============================================================================

/// List structure layout:
/// [type_tag: u64][length: u64][capacity: u64][data_ptr: *mut i64]
#[repr(C)]
pub struct MatterList {
    type_tag: u64, // 0x01 for List
    length: u64,
    capacity: u64,
    data_ptr: *mut i64,
}

/// Create a new list with given capacity
#[no_mangle]
pub extern "C" fn matter_list_new(capacity: usize) -> *mut MatterList {
    if capacity == 0 {
        return std::ptr::null_mut();
    }

    // Allocate list structure
    let list_ptr = matter_alloc(std::mem::size_of::<MatterList>()) as *mut MatterList;
    if list_ptr.is_null() {
        return std::ptr::null_mut();
    }

    // Allocate data array
    let data_size = capacity * std::mem::size_of::<i64>();
    let data_ptr = matter_alloc(data_size) as *mut i64;
    if data_ptr.is_null() {
        unsafe { matter_free(list_ptr as *mut u8, std::mem::size_of::<MatterList>()) };
        return std::ptr::null_mut();
    }

    // Initialize list
    unsafe {
        (*list_ptr).type_tag = 0x01;
        (*list_ptr).length = 0;
        (*list_ptr).capacity = capacity as u64;
        (*list_ptr).data_ptr = data_ptr;
    }

    list_ptr
}

/// Resize a list to new capacity
///
/// # Safety
///
/// `list_ptr` must be a valid MatterList pointer.
#[no_mangle]
pub unsafe extern "C" fn matter_list_resize(
    list_ptr: *mut MatterList,
    new_capacity: usize,
) -> bool {
    if list_ptr.is_null() || new_capacity == 0 {
        return false;
    }

    let list = unsafe { &mut *list_ptr };
    let old_capacity = list.capacity as usize;
    let old_data_ptr = list.data_ptr;

    // Allocate new data array
    let new_data_size = new_capacity * std::mem::size_of::<i64>();
    let new_data_ptr = matter_alloc(new_data_size) as *mut i64;
    if new_data_ptr.is_null() {
        return false;
    }

    // Copy old data
    let copy_count = std::cmp::min(list.length as usize, new_capacity);
    unsafe {
        std::ptr::copy_nonoverlapping(old_data_ptr, new_data_ptr, copy_count);
    }

    // Free old data
    let old_data_size = old_capacity * std::mem::size_of::<i64>();
    unsafe { matter_free(old_data_ptr as *mut u8, old_data_size) };

    // Update list
    list.data_ptr = new_data_ptr;
    list.capacity = new_capacity as u64;

    true
}

/// Free a list
///
/// # Safety
///
/// `list_ptr` must be a valid MatterList pointer.
#[no_mangle]
pub unsafe extern "C" fn matter_list_free(list_ptr: *mut MatterList) {
    if list_ptr.is_null() {
        return;
    }

    let list = unsafe { &*list_ptr };
    let data_size = list.capacity as usize * std::mem::size_of::<i64>();

    unsafe {
        matter_free(list.data_ptr as *mut u8, data_size);
        matter_free(list_ptr as *mut u8, std::mem::size_of::<MatterList>());
    }
}

/// Map structure layout:
/// [type_tag: u64][size: u64][buckets_ptr: *mut MapBucket]
#[repr(C)]
pub struct MatterMap {
    type_tag: u64, // 0x02 for Map
    size: u64,
    buckets_ptr: *mut MapBucket,
}

#[repr(C)]
pub struct MapBucket {
    key: i64,
    value: i64,
    next: *mut MapBucket,
}

/// Create a new map with 16 buckets
#[no_mangle]
pub extern "C" fn matter_map_new() -> *mut MatterMap {
    // Allocate map structure
    let map_ptr = matter_alloc(std::mem::size_of::<MatterMap>()) as *mut MatterMap;
    if map_ptr.is_null() {
        return std::ptr::null_mut();
    }

    // Allocate buckets array (16 buckets)
    let buckets_size = 16 * std::mem::size_of::<*mut MapBucket>();
    let buckets_ptr = matter_alloc(buckets_size) as *mut *mut MapBucket;
    if buckets_ptr.is_null() {
        unsafe { matter_free(map_ptr as *mut u8, std::mem::size_of::<MatterMap>()) };
        return std::ptr::null_mut();
    }

    // Initialize buckets to null
    unsafe {
        for i in 0..16 {
            *buckets_ptr.add(i) = std::ptr::null_mut();
        }
    }

    // Initialize map
    unsafe {
        (*map_ptr).type_tag = 0x02;
        (*map_ptr).size = 0;
        (*map_ptr).buckets_ptr = buckets_ptr as *mut MapBucket;
    }

    map_ptr
}

/// Hash function (FNV-1a)
#[no_mangle]
pub extern "C" fn matter_map_hash(key: i64) -> usize {
    let mut hash: u64 = 0xcbf29ce484222325;
    let bytes = key.to_le_bytes();

    for byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    (hash % 16) as usize
}

/// Insert key-value pair into map
///
/// # Safety
///
/// `map_ptr` must be a valid MatterMap pointer.
#[no_mangle]
pub unsafe extern "C" fn matter_map_insert(map_ptr: *mut MatterMap, key: i64, value: i64) -> bool {
    if map_ptr.is_null() {
        return false;
    }

    let map = unsafe { &mut *map_ptr };
    let bucket_idx = matter_map_hash(key);
    let buckets_ptr = map.buckets_ptr as *mut *mut MapBucket;

    // Update existing key in-place to avoid duplicate buckets and size inflation.
    let mut current = unsafe { *buckets_ptr.add(bucket_idx) };
    while !current.is_null() {
        unsafe {
            if (*current).key == key {
                (*current).value = value;
                return true;
            }
            current = (*current).next;
        }
    }

    // Allocate new bucket
    let new_bucket = matter_alloc(std::mem::size_of::<MapBucket>()) as *mut MapBucket;
    if new_bucket.is_null() {
        return false;
    }

    unsafe {
        (*new_bucket).key = key;
        (*new_bucket).value = value;
        (*new_bucket).next = *buckets_ptr.add(bucket_idx);
        *buckets_ptr.add(bucket_idx) = new_bucket;
    }

    map.size += 1;
    true
}

/// Lookup value by key in map
///
/// # Safety
///
/// `map_ptr` must be a valid MatterMap pointer.
/// Returns 0 if key not found.
#[no_mangle]
pub unsafe extern "C" fn matter_map_lookup(map_ptr: *mut MatterMap, key: i64) -> i64 {
    if map_ptr.is_null() {
        return 0;
    }

    let map = unsafe { &*map_ptr };
    let bucket_idx = matter_map_hash(key);
    let buckets_ptr = map.buckets_ptr as *mut *mut MapBucket;

    let mut current = unsafe { *buckets_ptr.add(bucket_idx) };
    while !current.is_null() {
        unsafe {
            if (*current).key == key {
                return (*current).value;
            }
            current = (*current).next;
        }
    }

    0 // Not found
}

/// Check if map has key
///
/// # Safety
///
/// `map_ptr` must be a valid MatterMap pointer.
#[no_mangle]
pub unsafe extern "C" fn matter_map_has(map_ptr: *mut MatterMap, key: i64) -> bool {
    if map_ptr.is_null() {
        return false;
    }

    let map = unsafe { &*map_ptr };
    let bucket_idx = matter_map_hash(key);
    let buckets_ptr = map.buckets_ptr as *mut *mut MapBucket;

    let mut current = unsafe { *buckets_ptr.add(bucket_idx) };
    while !current.is_null() {
        unsafe {
            if (*current).key == key {
                return true;
            }
            current = (*current).next;
        }
    }

    false
}

/// Return a list containing all map keys.
///
/// # Safety
///
/// `map_ptr` must be a valid MatterMap pointer.
#[no_mangle]
pub unsafe extern "C" fn matter_map_keys(map_ptr: *mut MatterMap) -> *mut MatterList {
    if map_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let map = unsafe { &*map_ptr };
    let list_ptr = matter_list_new(map.size as usize);
    if list_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let buckets_ptr = map.buckets_ptr as *mut *mut MapBucket;
    let mut pairs: Vec<(i64, i64)> = Vec::with_capacity(map.size as usize);

    for i in 0..16 {
        let mut current = unsafe { *buckets_ptr.add(i) };
        while !current.is_null() {
            unsafe {
                pairs.push(((*current).key, (*current).value));
                current = (*current).next;
            }
        }
    }

    // Deterministic output order for parity across backends.
    pairs.sort_by_key(|(key, _)| *key);

    for (write_idx, (key, _)) in pairs.iter().enumerate() {
        unsafe {
            *(*list_ptr).data_ptr.add(write_idx) = *key;
        }
    }

    unsafe {
        (*list_ptr).length = pairs.len() as u64;
    }

    list_ptr
}

/// Return a list containing all map values.
///
/// # Safety
///
/// `map_ptr` must be a valid MatterMap pointer.
#[no_mangle]
pub unsafe extern "C" fn matter_map_values(map_ptr: *mut MatterMap) -> *mut MatterList {
    if map_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let map = unsafe { &*map_ptr };
    let list_ptr = matter_list_new(map.size as usize);
    if list_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let buckets_ptr = map.buckets_ptr as *mut *mut MapBucket;
    let mut pairs: Vec<(i64, i64)> = Vec::with_capacity(map.size as usize);

    for i in 0..16 {
        let mut current = unsafe { *buckets_ptr.add(i) };
        while !current.is_null() {
            unsafe {
                pairs.push(((*current).key, (*current).value));
                current = (*current).next;
            }
        }
    }

    // Deterministic order matching keys() ordering.
    pairs.sort_by_key(|(key, _)| *key);

    for (write_idx, (_, value)) in pairs.iter().enumerate() {
        unsafe {
            *(*list_ptr).data_ptr.add(write_idx) = *value;
        }
    }

    unsafe {
        (*list_ptr).length = pairs.len() as u64;
    }

    list_ptr
}

/// Free a map
///
/// # Safety
///
/// `map_ptr` must be a valid MatterMap pointer.
#[no_mangle]
pub unsafe extern "C" fn matter_map_free(map_ptr: *mut MatterMap) {
    if map_ptr.is_null() {
        return;
    }

    let map = unsafe { &*map_ptr };
    let buckets_ptr = map.buckets_ptr as *mut *mut MapBucket;

    // Free all buckets
    for i in 0..16 {
        let mut current = unsafe { *buckets_ptr.add(i) };
        while !current.is_null() {
            let next = unsafe { (*current).next };
            unsafe { matter_free(current as *mut u8, std::mem::size_of::<MapBucket>()) };
            current = next;
        }
    }

    // Free buckets array
    let buckets_size = 16 * std::mem::size_of::<*mut MapBucket>();
    unsafe { matter_free(buckets_ptr as *mut u8, buckets_size) };

    // Free map structure
    unsafe { matter_free(map_ptr as *mut u8, std::mem::size_of::<MatterMap>()) };
}

/// Struct structure layout:
/// [type_tag: u64][type_id: u64][field_0: i64][field_1: i64]...
#[repr(C)]
pub struct MatterStruct {
    type_tag: u64, // 0x03 for Struct
    type_id: u64,
    // Fields follow...
}

/// Create a new struct with given field count
#[no_mangle]
pub extern "C" fn matter_struct_new(type_id: u64, field_count: usize) -> *mut MatterStruct {
    let size = std::mem::size_of::<MatterStruct>() + field_count * std::mem::size_of::<i64>();
    let struct_ptr = matter_alloc(size) as *mut MatterStruct;

    if !struct_ptr.is_null() {
        unsafe {
            (*struct_ptr).type_tag = 0x03;
            (*struct_ptr).type_id = type_id;
        }
    }

    struct_ptr
}

/// Free a struct
///
/// # Safety
///
/// `struct_ptr` must be a valid MatterStruct pointer.
/// `field_count` must match the count used in matter_struct_new.
#[no_mangle]
pub unsafe extern "C" fn matter_struct_free(struct_ptr: *mut MatterStruct, field_count: usize) {
    if struct_ptr.is_null() {
        return;
    }

    let size = std::mem::size_of::<MatterStruct>() + field_count * std::mem::size_of::<i64>();
    unsafe { matter_free(struct_ptr as *mut u8, size) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_int() {
        matter_print_int(42);
        // Should not panic
    }

    #[test]
    fn test_print_bool() {
        matter_print_bool(true);
        matter_print_bool(false);
        // Should not panic
    }

    #[test]
    fn test_alloc_free() {
        let ptr = matter_alloc(1024);
        assert!(!ptr.is_null());
        unsafe { matter_free(ptr, 1024) };
        // Should not panic
    }

    #[test]
    fn test_list_new_free() {
        let list = matter_list_new(10);
        assert!(!list.is_null());
        unsafe {
            assert_eq!((*list).type_tag, 0x01);
            assert_eq!((*list).length, 0);
            assert_eq!((*list).capacity, 10);
            matter_list_free(list);
        }
    }

    #[test]
    fn test_list_resize() {
        let list = matter_list_new(5);
        assert!(!list.is_null());

        unsafe {
            // Add some data
            (*list).length = 3;
            *(*list).data_ptr.add(0) = 10;
            *(*list).data_ptr.add(1) = 20;
            *(*list).data_ptr.add(2) = 30;

            // Resize
            assert!(matter_list_resize(list, 10));
            assert_eq!((*list).capacity, 10);
            assert_eq!((*list).length, 3);

            // Check data preserved
            assert_eq!(*(*list).data_ptr.add(0), 10);
            assert_eq!(*(*list).data_ptr.add(1), 20);
            assert_eq!(*(*list).data_ptr.add(2), 30);

            matter_list_free(list);
        }
    }

    #[test]
    fn test_map_new_free() {
        let map = matter_map_new();
        assert!(!map.is_null());
        unsafe {
            assert_eq!((*map).type_tag, 0x02);
            assert_eq!((*map).size, 0);
            matter_map_free(map);
        }
    }

    #[test]
    fn test_map_insert_lookup() {
        let map = matter_map_new();
        assert!(!map.is_null());

        unsafe {
            // Insert key-value pairs
            assert!(matter_map_insert(map, 1, 100));
            assert!(matter_map_insert(map, 2, 200));
            assert!(matter_map_insert(map, 3, 300));

            // Lookup
            assert_eq!(matter_map_lookup(map, 1), 100);
            assert_eq!(matter_map_lookup(map, 2), 200);
            assert_eq!(matter_map_lookup(map, 3), 300);
            assert_eq!(matter_map_lookup(map, 999), 0); // Not found

            // Has
            assert!(matter_map_has(map, 1));
            assert!(matter_map_has(map, 2));
            assert!(!matter_map_has(map, 999));

            matter_map_free(map);
        }
    }

    #[test]
    fn test_map_insert_overwrite_keeps_size() {
        let map = matter_map_new();
        assert!(!map.is_null());

        unsafe {
            assert!(matter_map_insert(map, 42, 100));
            assert_eq!((*map).size, 1);
            assert_eq!(matter_map_lookup(map, 42), 100);

            // Overwrite same key: value changes, size must stay stable.
            assert!(matter_map_insert(map, 42, 999));
            assert_eq!((*map).size, 1);
            assert_eq!(matter_map_lookup(map, 42), 999);

            matter_map_free(map);
        }
    }

    #[test]
    fn test_map_keys_and_values() {
        let map = matter_map_new();
        assert!(!map.is_null());

        unsafe {
            assert!(matter_map_insert(map, 1, 100));
            assert!(matter_map_insert(map, 2, 200));
            assert!(matter_map_insert(map, 3, 300));

            let keys = matter_map_keys(map);
            let values = matter_map_values(map);

            assert!(!keys.is_null());
            assert!(!values.is_null());
            assert_eq!((*keys).length, 3);
            assert_eq!((*values).length, 3);

            matter_list_free(keys);
            matter_list_free(values);
            matter_map_free(map);
        }
    }

    #[test]
    fn test_map_keys_and_values_are_deterministic_and_aligned() {
        let map = matter_map_new();
        assert!(!map.is_null());

        unsafe {
            // Insert intentionally out of order
            assert!(matter_map_insert(map, 30, 3000));
            assert!(matter_map_insert(map, 10, 1000));
            assert!(matter_map_insert(map, 20, 2000));

            let keys = matter_map_keys(map);
            let values = matter_map_values(map);
            assert!(!keys.is_null());
            assert!(!values.is_null());
            assert_eq!((*keys).length, 3);
            assert_eq!((*values).length, 3);

            let k0 = *(*keys).data_ptr.add(0);
            let k1 = *(*keys).data_ptr.add(1);
            let k2 = *(*keys).data_ptr.add(2);
            assert_eq!((k0, k1, k2), (10, 20, 30));

            let v0 = *(*values).data_ptr.add(0);
            let v1 = *(*values).data_ptr.add(1);
            let v2 = *(*values).data_ptr.add(2);
            assert_eq!((v0, v1, v2), (1000, 2000, 3000));

            matter_list_free(keys);
            matter_list_free(values);
            matter_map_free(map);
        }
    }

    #[test]
    fn test_struct_new_free() {
        let struct_ptr = matter_struct_new(12345, 2);
        assert!(!struct_ptr.is_null());
        unsafe {
            assert_eq!((*struct_ptr).type_tag, 0x03);
            assert_eq!((*struct_ptr).type_id, 12345);
            matter_struct_free(struct_ptr, 2);
        }
    }
}
