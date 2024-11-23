#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::panic::PanicInfo;
use core::cell::UnsafeCell;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// Bloc de mémoire libre
#[repr(C)]
pub struct Block {
    pub size: usize,              // Taille du bloc
    pub next: *mut Block,         // Pointeur vers le prochain bloc
}

impl Block {
    /// Retourne l'adresse de début de ce bloc
    fn starting_addr(&self) -> usize {
        self as *const Block as usize
    }

    /// Retourne l'adresse de fin de ce bloc
    fn finishing_addr(&self) -> usize {
        self.starting_addr() + self.size
    }
}

// Allocateur FreeList
pub struct FreeListAllocator {
    free_list: UnsafeCell<*mut Block>, // Liste des blocs libres (pointeur brut)
}

unsafe impl Sync for FreeListAllocator {}

unsafe impl GlobalAlloc for FreeListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = (layout.size(), layout.align());
        let allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_block(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("Overflow");
            let excess_size = (*region).finishing_addr() - alloc_end;

            if excess_size > 0 {
                allocator.insert_free_region(alloc_end, excess_size);
            }

            alloc_start as *mut u8
        } else {
            null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size();
        let allocator = self.lock();
        allocator.insert_free_region(ptr as usize, size);
        allocator.merge_free_blocks();
    }
}

impl FreeListAllocator {
    /// Retourne une référence mutable à l'allocateur en utilisant `UnsafeCell`
    pub unsafe fn lock(&self) -> &mut FreeListAllocator {
        // Utilisation correcte de `UnsafeCell` pour accéder à la mutabilité interne
        &mut *(self as *const _ as *mut FreeListAllocator)
    }

    /// Cherche un bloc libre correspondant à une taille et un alignement donnés
    pub unsafe fn find_block(&mut self, size: usize, alignment: usize) -> Option<(*mut Block, usize)> {
        let mut current_block = *self.free_list.get();
        let mut previous_block: *mut Block = null_mut();

        while !current_block.is_null() {
            if let Ok(allocation_address) = Self::check_block_allocation(current_block, size, alignment) {
                if !previous_block.is_null() {
                    (*previous_block).next = (*current_block).next;
                } else {
                    *self.free_list.get() = (*current_block).next;
                }

                return Some((current_block, allocation_address));
            }

            previous_block = current_block;
            current_block = (*current_block).next;
        }

        None
    }

    /// Vérifie si un bloc peut être utilisé pour une allocation
    pub unsafe fn check_block_allocation(block: *mut Block, size: usize, alignment: usize) -> Result<usize, ()> {
        let start_address = (*block).starting_addr();
        let aligned_address = (start_address + alignment - 1) & !(alignment - 1);

        if aligned_address + size <= (*block).finishing_addr() {
            Ok(aligned_address)
        } else {
            Err(())
        }
    }

    /// Insère une région mémoire libre dans la liste chaînée
    pub unsafe fn insert_free_region(&self, addr: usize, size: usize) {
        let alignment = core::mem::align_of::<Block>();

        if size < core::mem::size_of::<Block>() || addr % alignment != 0 {
            return;
        }

        let new_block = addr as *mut Block;
        (*new_block).size = size;

        (*new_block).next = *self.free_list.get();
        *self.free_list.get() = new_block;
    }

    /// Fusion des  sb blocs libres adjacents
    pub unsafe fn merge_free_blocks(&mut self) {
        let mut current_block = *self.free_list.get();

        while !current_block.is_null() {
            let next_block = (*current_block).next;

            if (*current_block).finishing_addr() && !next_block.is_null() == next_block as usize {
                
                (*current_block).size += (*next_block).size;
                
                (*current_block).next = (*next_block).next;
            } else {
                
                
                current_block = next_block;
            }
        }
    }

    /// Initialise l'allocateur
    pub unsafe fn init(&self, heap_start: usize, heap_size: usize) {
        self.insert_free_region(heap_start, heap_size);
    }
}

// Déclaration de l'allocateur global
#[global_allocator]
static ALLOCATOR: FreeListAllocator = FreeListAllocator {
    free_list: UnsafeCell::new(null_mut()),
};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    static mut HEAP: [u8; 1024] = [0; 1024];

    unsafe {
        ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP.len());
    }

    loop {}
}
