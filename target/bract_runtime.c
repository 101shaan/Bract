#include "bract_runtime.h"
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

// Static variables
static prism_error_code_t g_error_code = PRISM_ERROR_NONE;
static char g_error_message[256] = {0};

// Memory Management Implementation
void* prism_alloc(size_t size)
{
    void* ptr = malloc(size);
    if (!ptr && size > 0) {
        prism_panic("Out of memory");
    }
    return ptr;
}
void* prism_realloc(void* ptr, size_t new_size)
{
    void* new_ptr = realloc(ptr, new_size);
    if (!new_ptr && new_size > 0) {
        prism_panic("Out of memory");
    }
    return new_ptr;
}
void prism_free(void* ptr)
{
    if (ptr) {
        free(ptr);
    }
}
void* prism_calloc(size_t count, size_t size)
{
    void* ptr = calloc(count, size);
    if (!ptr && count > 0 && size > 0) {
        prism_panic("Out of memory");
    }
    return ptr;
}
// String Handling Implementation
prism_str_t prism_str_new(const char* cstr)
{
    prism_str_t str = {0};
    if (cstr) {
        str.length = strlen(cstr);
        str.capacity = str.length + 1;
        str.data = prism_alloc(str.capacity);
        memcpy(str.data, cstr, str.length);
        str.data[str.length] = '\0';
        str.ref_count = 1;
    }
    return str;
}
bool prism_str_eq(const prism_str_t* a, const prism_str_t* b)
{
    if (!a || !b) return false;
    if (a->length != b->length) return false;
    return memcmp(a->data, b->data, a->length) == 0;
}
void prism_str_free(prism_str_t* str)
{
    if (str && str->data) {
        prism_free(str->data);
        str->data = NULL;
        str->length = 0;
        str->capacity = 0;
        str->ref_count = 0;
    }
}
// Array Handling Implementation
prism_array_t prism_array_new(size_t element_size)
{
    prism_array_t arr = {0};
    arr.element_size = element_size;
    arr.capacity = 4; // Start with small capacity
    arr.data = prism_alloc(arr.capacity * element_size);
    arr.ref_count = 1;
    return arr;
}
void prism_array_free(prism_array_t* arr)
{
    if (arr && arr->data) {
        prism_free(arr->data);
        arr->data = NULL;
        arr->length = 0;
        arr->capacity = 0;
        arr->ref_count = 0;
    }
}
// Error Handling Implementation
void prism_panic(const char* message)
{
    fprintf(stderr, "Prism panic: %s\n", message ? message : "unknown error");
    abort();
}
void prism_set_error(prism_error_code_t code, const char* message)
{
    g_error_code = code;
    if (message) {
        strncpy(g_error_message, message, sizeof(g_error_message) - 1);
        g_error_message[sizeof(g_error_message) - 1] = '\0';
    } else {
        g_error_message[0] = '\0';
    }
}
prism_error_code_t prism_get_error(void)
{
    return g_error_code;
}
const char* prism_get_error_message(void)
{
    return g_error_message;
}
// I/O Operations Implementation
void prism_print_str(const prism_str_t* str)
{
    if (str && str->data) {
        printf("%.*s", (int)str->length, str->data);
    }
}
void prism_print_cstr(const char* cstr)
{
    if (cstr) {
        printf("%s", cstr);
    }
}
void prism_print_int(int64_t value)
{
    printf("%lld", (long long)value);
}
void prism_println(void)
{
    printf("\n");
}
// Utility Functions Implementation
prism_range_t prism_range(int64_t start, int64_t end, bool inclusive)
{
    prism_range_t range = { start, end, inclusive };
    return range;
}
prism_optional_t prism_some(void* value)
{
    prism_optional_t opt = { value, true };
    return opt;
}
prism_optional_t prism_none(void)
{
    prism_optional_t opt = { NULL, false };
    return opt;
}
prism_result_t prism_ok(void* value)
{
    prism_result_t result = { value, NULL, true };
    return result;
}
prism_result_t prism_err(void* error)
{
    prism_result_t result = { NULL, error, false };
    return result;
}
