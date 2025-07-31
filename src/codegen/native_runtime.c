// Ultra-minimal native runtime for Bract - ZERO external dependencies
// Completely self-contained - no libc, no system calls

// basic heap - super simple bump allocator
static char heap[1024 * 1024]; // 1MB heap
static unsigned long heap_pos = 0;

// minimal malloc - just bump allocator
void* bract_malloc(unsigned long size) {
    if (heap_pos + size >= sizeof(heap)) {
        return 0; // out of memory
    }
    void* ptr = &heap[heap_pos];
    heap_pos += size;
    return ptr;
}

// minimal free - do nothing (bump allocator)
void bract_free(void* ptr) {
    // no-op for bump allocator
    (void)ptr;
}

// minimal reference counting - just counters
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