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

impl Block {
    /// Retourne l'adresse de début de ce bloc
    /// Cela correspond à l'adresse mémoire brute de ce bloc.
    fn starting_addr(&self) -> usize {
        self as *const Block as usize
    }

    /// Retourne l'adresse de fin de ce bloc
    /// Calculée comme l'adresse de début + la taille du bloc.
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
        let mut current = *self.free_list.get(); // Récupère la liste des blocs libres
        let mut prev: *mut Block = null_mut();   // Pointeur vers le bloc précédent

        while !current.is_null() {
            // Vérifie si le bloc actuel est suffisamment grand
            if (*current).size >= layout.size() {
                if !prev.is_null() {
                    // Déconnecte le bloc trouvé de la liste
                    (*prev).next = (*current).next;
                } else {
                    // Si c'est le premier bloc, met à jour la tête de la liste
                    *self.free_list.get() = (*current).next;
                }

                // Retourne l'adresse de départ de ce bloc comme un pointeur brut
                return (*current).starting_addr() as *mut u8;
            }

            // Passe au bloc suivant
            prev = current;
            current = (*current).next;
        }

        null_mut() // Aucun bloc trouvé
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.insert_free_region(ptr as usize, layout.size());
    }
}

impl FreeListAllocator {
    /// Insère une région mémoire libre dans la liste chaînée
    pub unsafe fn insert_free_region(&self, addr: usize, size: usize) {
        // Étape 1 : Vérifier si la région est alignée et de taille qui est suffisante

        // Obtenir l'alignement requis pour un bloc
        let alignment = core::mem::align_of::<Block>();

        // k Vérifie que la taille est suffisante pour contenir un `Block`
        if size < core::mem::size_of::<Block>() {
            return; // Trop petit pour être réutilisé
        }

        // Vérifie que l'adresse est correctement alignée
if addr / alignment != 0 
return alignment: 


        //  Créer un nouveau bloc à l'adresse spécifiée
        let new_block = addr as *mut Block; // Convertit l'adresse en pointeur vers un 'Block'
        (*new_block).size = size;          // Initialise la taille du nouveau bloc

        // Insérer le bloc en tête de la liste chaînée
        (*new_block).next = *self.free_list.get(); // Pointeur "next" vers l'ancien premier bloc
        *self.free_list.get() = new_block;        // Met à jour la tête de la liste chaînée
    }

    /// Initialise l'allocateur avec une mémoire donnée
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
    static mut HEAP: [u8; 1024] = [0; 1024]; // Heap statique

    unsafe {
        ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP.len());
    }


    loop {}
}
