#include "prism_runtime.h"
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <math.h>

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
void prism_ref_inc(void* ptr)
{
    // TODO: Implement reference counting
    (void)ptr;
}
void prism_ref_dec(void* ptr)
{
    // TODO: Implement reference counting
    (void)ptr;
}
uint32_t prism_ref_count(void* ptr)
{
    // TODO: Implement reference counting
    (void)ptr;
    return 1;
}
#ifdef PRISM_DEBUG
void prism_memory_report(void)
{
    // TODO: Implement memory usage reporting
    printf("Memory usage reporting not implemented yet\n");
}
size_t prism_memory_usage(void)
{
    // TODO: Implement memory usage tracking
    return 0;
}
#endif
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
prism_str_t prism_str_from_bytes(const char* bytes, size_t length)
{
    prism_str_t str = {0};
    if (bytes && length > 0) {
        str.length = length;
        str.capacity = length + 1;
        str.data = prism_alloc(str.capacity);
        memcpy(str.data, bytes, length);
        str.data[length] = '\0';
        str.ref_count = 1;
    }
    return str;
}
prism_str_t prism_str_with_capacity(size_t capacity)
{
    prism_str_t str = {0};
    if (capacity > 0) {
        str.capacity = capacity;
        str.data = prism_alloc(capacity);
        str.data[0] = '\0';
        str.ref_count = 1;
    }
    return str;
}
prism_str_t prism_str_clone(const prism_str_t* str)
{
    prism_str_t clone = {0};
    if (str && str->data) {
        clone.length = str->length;
        clone.capacity = str->capacity;
        clone.data = prism_alloc(clone.capacity);
        memcpy(clone.data, str->data, str->length + 1);
        clone.ref_count = 1;
    }
    return clone;
}
void prism_str_push(prism_str_t* str, char ch)
{
    if (!str) return;
    if (str->length + 1 >= str->capacity) {
        size_t new_capacity = str->capacity > 0 ? str->capacity * 2 : 8;
        str->data = prism_realloc(str->data, new_capacity);
        str->capacity = new_capacity;
    }
    str->data[str->length] = ch;
    str->length++;
    str->data[str->length] = '\0';
}
void prism_str_push_str(prism_str_t* str, const prism_str_t* other)
{
    if (!str || !other || !other->data) return;
    size_t new_length = str->length + other->length;
    if (new_length + 1 > str->capacity) {
        size_t new_capacity = str->capacity;
        while (new_capacity <= new_length) {
            new_capacity = new_capacity > 0 ? new_capacity * 2 : 8;
        }
        str->data = prism_realloc(str->data, new_capacity);
        str->capacity = new_capacity;
    }
    memcpy(str->data + str->length, other->data, other->length);
    str->length = new_length;
    str->data[str->length] = '\0';
}
void prism_str_push_cstr(prism_str_t* str, const char* cstr)
{
    if (!str || !cstr) return;
    size_t cstr_len = strlen(cstr);
    size_t new_length = str->length + cstr_len;
    if (new_length + 1 > str->capacity) {
        size_t new_capacity = str->capacity;
        while (new_capacity <= new_length) {
            new_capacity = new_capacity > 0 ? new_capacity * 2 : 8;
        }
        str->data = prism_realloc(str->data, new_capacity);
        str->capacity = new_capacity;
    }
    memcpy(str->data + str->length, cstr, cstr_len);
    str->length = new_length;
    str->data[str->length] = '\0';
}
prism_str_t prism_str_concat(const prism_str_t* a, const prism_str_t* b)
{
    prism_str_t result = {0};
    if (!a && !b) return result;
    if (!a) return prism_str_clone(b);
    if (!b) return prism_str_clone(a);
    
    result.length = a->length + b->length;
    result.capacity = result.length + 1;
    result.data = prism_alloc(result.capacity);
    result.ref_count = 1;
    
    memcpy(result.data, a->data, a->length);
    memcpy(result.data + a->length, b->data, b->length);
    result.data[result.length] = '\0';
    
    return result;
}
bool prism_str_eq(const prism_str_t* a, const prism_str_t* b)
{
    if (!a || !b) return false;
    if (a->length != b->length) return false;
    return memcmp(a->data, b->data, a->length) == 0;
}
bool prism_str_eq_cstr(const prism_str_t* str, const char* cstr)
{
    if (!str || !cstr) return false;
    size_t cstr_len = strlen(cstr);
    if (str->length != cstr_len) return false;
    return memcmp(str->data, cstr, str->length) == 0;
}
int prism_str_cmp(const prism_str_t* a, const prism_str_t* b)
{
    if (!a || !b) return 0;
    size_t min_len = a->length < b->length ? a->length : b->length;
    int result = memcmp(a->data, b->data, min_len);
    if (result != 0) return result;
    if (a->length < b->length) return -1;
    if (a->length > b->length) return 1;
    return 0;
}
size_t prism_str_len(const prism_str_t* str)
{
    return str ? str->length : 0;
}
bool prism_str_is_empty(const prism_str_t* str)
{
    return !str || str->length == 0;
}
const char* prism_str_as_cstr(const prism_str_t* str)
{
    return str && str->data ? str->data : "";
}
void prism_str_clear(prism_str_t* str)
{
    if (str && str->data) {
        str->length = 0;
        str->data[0] = '\0';
    }
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
prism_array_t prism_array_with_capacity(size_t element_size, size_t capacity)
{
    prism_array_t arr = {0};
    arr.element_size = element_size;
    arr.capacity = capacity > 0 ? capacity : 4;
    arr.data = prism_alloc(arr.capacity * element_size);
    arr.ref_count = 1;
    return arr;
}
prism_array_t prism_array_clone(const prism_array_t* arr)
{
    prism_array_t clone = {0};
    if (arr && arr->data) {
        clone.element_size = arr->element_size;
        clone.capacity = arr->capacity;
        clone.length = arr->length;
        clone.data = prism_alloc(clone.capacity * clone.element_size);
        memcpy(clone.data, arr->data, arr->length * arr->element_size);
        clone.ref_count = 1;
    }
    return clone;
}
void prism_array_push(prism_array_t* arr, const void* element)
{
    if (!arr || !element) return;
    if (arr->length >= arr->capacity) {
        size_t new_capacity = arr->capacity * 2;
        arr->data = prism_realloc(arr->data, new_capacity * arr->element_size);
        arr->capacity = new_capacity;
    }
    char* dest = (char*)arr->data + (arr->length * arr->element_size);
    memcpy(dest, element, arr->element_size);
    arr->length++;
}
bool prism_array_pop(prism_array_t* arr, void* element)
{
    if (!arr || arr->length == 0) return false;
    arr->length--;
    if (element) {
        char* src = (char*)arr->data + (arr->length * arr->element_size);
        memcpy(element, src, arr->element_size);
    }
    return true;
}
void* prism_array_get(const prism_array_t* arr, size_t index)
{
    if (!arr || index >= arr->length) return NULL;
    return (char*)arr->data + (index * arr->element_size);
}
bool prism_array_set(prism_array_t* arr, size_t index, const void* element)
{
    if (!arr || !element || index >= arr->length) return false;
    char* dest = (char*)arr->data + (index * arr->element_size);
    memcpy(dest, element, arr->element_size);
    return true;
}
size_t prism_array_len(const prism_array_t* arr)
{
    return arr ? arr->length : 0;
}
bool prism_array_is_empty(const prism_array_t* arr)
{
    return !arr || arr->length == 0;
}
void prism_array_clear(prism_array_t* arr)
{
    if (arr) {
        arr->length = 0;
    }
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
void prism_panic_fmt(const char* format, ...)
{
    va_list args;
    fprintf(stderr, "Prism panic: ");
    va_start(args, format);
    vfprintf(stderr, format, args);
    va_end(args);
    fprintf(stderr, "\n");
    abort();
}
void prism_assert(bool condition, const char* message)
{
    if (!condition) {
        prism_panic(message ? message : "Assertion failed");
    }
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
void prism_clear_error(void)
{
    g_error_code = PRISM_ERROR_NONE;
    g_error_message[0] = '\0';
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
void prism_print_uint(uint64_t value)
{
    printf("%llu", (unsigned long long)value);
}
void prism_print_float(double value)
{
    printf("%g", value);
}
void prism_print_bool(bool value)
{
    printf("%s", value ? "true" : "false");
}
void prism_println(void)
{
    printf("\n");
    fflush(stdout);
}
prism_str_t prism_read_line(void)
{
    prism_str_t result = prism_str_with_capacity(256);
    char buffer[256];
    if (fgets(buffer, sizeof(buffer), stdin)) {
        // Remove newline if present
        size_t len = strlen(buffer);
        if (len > 0 && buffer[len-1] == '\n') {
            buffer[len-1] = '\0';
            len--;
        }
        prism_str_push_cstr(&result, buffer);
    }
    return result;
}
bool prism_read_int(int64_t* value)
{
    if (!value) return false;
    long long temp;
    if (scanf("%lld", &temp) == 1) {
        *value = (int64_t)temp;
        return true;
    }
    return false;
}
bool prism_read_float(double* value)
{
    if (!value) return false;
    return scanf("%lf", value) == 1;
}
// Utility Functions Implementation
prism_range_t prism_range(int64_t start, int64_t end, bool inclusive)
{
    prism_range_t range = { start, end, inclusive };
    return range;
}
bool prism_range_contains(const prism_range_t* range, int64_t value)
{
    if (!range) return false;
    if (range->inclusive) {
        return value >= range->start && value <= range->end;
    } else {
        return value >= range->start && value < range->end;
    }
}
int64_t prism_range_len(const prism_range_t* range)
{
    if (!range) return 0;
    int64_t len = range->end - range->start;
    if (range->inclusive) len++;
    return len > 0 ? len : 0;
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
bool prism_optional_is_some(const prism_optional_t* opt)
{
    return opt && opt->has_value;
}
bool prism_optional_is_none(const prism_optional_t* opt)
{
    return !opt || !opt->has_value;
}
void* prism_optional_unwrap(const prism_optional_t* opt)
{
    if (!opt || !opt->has_value) {
        prism_panic("Attempted to unwrap None value");
    }
    return opt->data;
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
bool prism_result_is_ok(const prism_result_t* result)
{
    return result && result->is_ok;
}
bool prism_result_is_err(const prism_result_t* result)
{
    return result && !result->is_ok;
}
void* prism_result_unwrap(const prism_result_t* result)
{
    if (!result || !result->is_ok) {
        prism_panic("Attempted to unwrap error result");
    }
    return result->data;
}
int64_t prism_abs_int(int64_t value)
{
    return value < 0 ? -value : value;
}
double prism_abs_float(double value)
{
    return fabs(value);
}
int64_t prism_min_int(int64_t a, int64_t b)
{
    return a < b ? a : b;
}
int64_t prism_max_int(int64_t a, int64_t b)
{
    return a > b ? a : b;
}
double prism_min_float(double a, double b)
{
    return fmin(a, b);
}
double prism_max_float(double a, double b)
{
    return fmax(a, b);
}
double prism_pow(double base, double exponent)
{
    return pow(base, exponent);
}
double prism_sqrt(double value)
{
    if (value < 0) {
        prism_set_error(PRISM_ERROR_INVALID_ARGUMENT, "Cannot take square root of negative number");
        return 0.0;
    }
    return sqrt(value);
}
double prism_sin(double radians)
{
    return sin(radians);
}
double prism_cos(double radians)
{
    return cos(radians);
}
double prism_tan(double radians)
{
    return tan(radians);
}
