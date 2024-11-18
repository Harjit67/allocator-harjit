#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{null_mut};
use core::panic::PanicInfo;
use core::cell::UnsafeCell;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// Bloc de mémoire libre
#[repr(C)]
struct Block {
    size: usize,              // Taille du bloc
    next: *mut Block,         // Pointeur vers le prochain bloc
}

// Allocateur FreeList
pub struct FreeListAllocator {
    free_list: UnsafeCell<*mut Block>, // Liste des blocs libres (pointeur brut)
}

unsafe impl Sync for FreeListAllocator {}

unsafe impl GlobalAlloc for FreeListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut current = *self.free_list.get(); // Récupère la liste des blocs libres
        let mut prev: *mut Block = null_mut();   // Pointeur vers le bloc précédent

        while !current.is_null() {
            if (*current).size >= layout.size() {
                // Si on trouve un bloc suffisant
                if !prev.is_null() {
                    // Déconnecte le bloc trouvé
                    (*prev).next = (*current).next;
                } else {
                    // Si c'est le premier bloc, met à jour la liste libre
                    *self.free_list.get() = (*current).next;
                }
                return current as *mut u8; // Retourne le pointeur brut
            }
            prev = current;
            current = (*current).next;
        }

        null_mut() // Aucun bloc trouvé
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let block = ptr as *mut Block;
        (*block).size = layout.size();
        (*block).next = *self.free_list.get(); // Ajoute le bloc à la liste libre
        *self.free_list.get() = block;
    }
}

// Déclaration de l'allocateur global
#[global_allocator]
static ALLOCATOR: FreeListAllocator = FreeListAllocator {
    free_list: UnsafeCell::new(null_mut()),
};

impl FreeListAllocator {
    pub unsafe fn init(&self, heap_start: usize, heap_size: usize) {
        let block = heap_start as *mut Block;
        (*block).size = heap_size;
        (*block).next = null_mut(); // Initialise le premier bloc
        *self.free_list.get() = block; // Définit le bloc comme le premier dans la liste libre
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    static mut HEAP: [u8; 1024] = [0; 1024]; // Heap statique

    unsafe {
        ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP.len());
    }

    loop {}
}
