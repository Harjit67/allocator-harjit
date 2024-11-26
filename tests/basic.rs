use allocateurharjit::ALLOCATOR;
use core::alloc::Layout;

#[test]
fn test_allocator() {
    unsafe {
        static mut HEAP: [u8; 1024] = [0; 1024];
        ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP.len());

        let layout = Layout::from_size_align(16, 8).unwrap();
        let ptr = ALLOCATOR.alloc(layout);
        assert!(!ptr.is_null(), "L'allocation a échoué.");

        ALLOCATOR.dealloc(ptr, layout);
    }
}
