// Ultra-minimal native runtime for Bract - no external dependencies
// Just enough to get linking working

#include <stdint.h>

// basic heap - super simple bump allocator for now
static char heap[1024 * 1024]; // 1MB heap
static size_t heap_pos = 0;

// minimal malloc - just bump allocator
void* bract_malloc(size_t size) {
    if (heap_pos + size >= sizeof(heap)) {
        return 0; // out of memory
    }
    void* ptr = &heap[heap_pos];
    heap_pos += size;
    return ptr;
}

// minimal free - do nothing for now (bump allocator)
void bract_free(void* ptr) {
    // no-op for bump allocator
    (void)ptr;
}

// minimal reference counting - just counters for now
void bract_arc_inc(int* refcount) {
    if (refcount) {
        (*refcount)++;
    }
}

void bract_arc_dec(int* refcount) {
    if (refcount && *refcount > 0) {
        (*refcount)--;
    }
}