#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::panic::PanicInfo;
use core::cell::UnsafeCell;
use core::mem;


#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}


// Représente un bloc de mémoire libre dans la liste chaînée.

// Ce bloc est utilisé par l'allocateur pour suivre les régions
// de mémoire non utilisées.


#[repr(C)]
struct Block {
    size: usize,              // Taille du bloc
    next: *mut Block,         // Pointeur vers le prochain bloc
}

impl Block {
    /// Retourne l'adresse de début de ce bloc.
    fn starting_addr(&self) -> usize {
        self as *const Block as usize
    }

    /// Retourne l'adresse de fin de ce bloc.
    fn finishing_addr(&self) -> usize {
        self.starting_addr() + self.size
    }
}


// Un allocateur basé sur une liste chaînée de blocs libres.

// Cet allocateur suit une stratégie simple : trouver un bloc
// libre qui peut satisfaire une demande d'allocation et le decouper si nécessaire.

// Allocateur FreeList
pub struct FreeListAllocator {
    free_list: UnsafeCell<*mut Block>, // Liste des blocs libres (pointeur brut)
}

/// # Safety
/// Cette implémentation de `GlobalAlloc` doit garantir que :
/// - `alloc` retourne une région mémoire correctement alignée.
/// - `dealloc` libère uniquement les blocs préalablement alloués par cet allocateur.
/// - Les opérations de modification sur la liste des blocs libres respectent les règles d'accès concurrent.
unsafe impl Sync for FreeListAllocator {}

unsafe impl GlobalAlloc for FreeListAllocator {
    /// # Safety
    /// Cette méthode est marquée `unsafe` car elle effectue des opérations de bas niveau
    /// pour allouer de la mémoire brute. L'appelant doit garantir que :
    /// - Le `Layout` fourni est valide.
    /// - La mémoire retournée est utilisée conformément aux règles du `Layout`.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (adjusted_size, alignment) = Self::adjust_layout(layout); // Ajustement du layout
        let mut current = *self.free_list.get(); // Récupère la liste des blocs libres
        let mut previous_block: *mut Block = null_mut(); // Pointeur vers le bloc précédent

        while !current.is_null() {
            if (*current).size >= adjusted_size {
                if !previous_block.is_null() {
                    (*previous_block).next = (*current).next;
                } else {
                    *self.free_list.get() = (*current).next;
                }

                return (*current).starting_addr() as *mut u8;
            }

            previous_block = current;
            current = (*current).next;
        }

        null_mut()
    }

    /// # Safety
    /// Cette méthode est `unsafe` car elle manipule directement les pointeurs
    /// et nécessite que l'appelant garantisse :
    /// - Que `ptr` pointe vers une région valide allouée par cet allocateur.
    /// - Que la taille et l'alignement fournis dans `Layout` correspondent à ceux utilisés lors de l'allocation.
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (adjusted_size, _) = Self::adjust_layout(layout); // Ajustement du layout
        self.insert_free_region(ptr as usize, adjusted_size);
    }
}

impl FreeListAllocator {
    /// Ajuste la taille et l'alignement pour répondre aux contraintes minimales.
    fn adjust_layout(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<Block>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<Block>());
        (size, layout.align())
    }

    /// # Safety
    /// Cette méthode est `unsafe` car elle accède et modifie directement la liste des blocs libres.
    /// L'appelant doit garantir que la liste est dans un état cohérent avant l'appel.
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

    /// # Safety
    /// Vérifie si un bloc peut être utilisé pour une allocation. Cette méthode est `unsafe` car elle
    /// manipule directement les pointeurs et nécessite que `block` pointe vers un bloc valide.
    pub unsafe fn check_block_allocation(block: *mut Block, size: usize, alignment: usize) -> Result<usize, ()> {
        let start_address = (*block).starting_addr();
        let aligned_address = (start_address + alignment - 1) & !(alignment - 1);

        if aligned_address + size <= (*block).finishing_addr() {
            Ok(aligned_address)
        } else {
            Err(())
        }
    }

    /// # Safety
    /// Insère une région mémoire libre dans la liste chaînée. L'appelant doit garantir que :
    /// - `addr` est aligné correctement.
    /// - La taille de la région est suffisante pour contenir un bloc.
    pub unsafe fn insert_free_region(&self, addr: usize, size: usize) {
        let alignment = mem::align_of::<Block>();

        if size < mem::size_of::<Block>() || addr % alignment != 0 {
            return;
        }

        let new_block = addr as *mut Block;
        (*new_block).size = size;

        (*new_block).next = *self.free_list.get();
        *self.free_list.get() = new_block;
    }

    /// # Safety
    /// Initialise l'allocateur en insérant une région mémoire libre couvrant
    /// la totalité de l'espace mémoire disponible.
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
        /// On initialise l'allocateur avec un tas de 1024 octets.
        /// Cette opération est sûre car le tableau est correctement aligné.
        ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP.len());
    }

    loop {}
}
