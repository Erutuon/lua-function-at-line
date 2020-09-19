#ifndef LUA_FUNCTION_AT_LINE_H
#define LUA_FUNCTION_AT_LINE_H

#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

// Object containing information on which functions are at given lines in a Lua module.
// Must be allocated and freed by Rust.
typedef struct lua_module_function_lines lua_module_function_lines;

// Generate new object from Lua code. Provide length of memory area pointed to by `code` in `code_len`.
// Call `lua_module_function_lines_free` to free it.
lua_module_function_lines * lua_module_function_lines_new(const char * code, size_t code_len);

// Gets name of function at `line` (zero-indexed). Provides length of function name in `name_len`.
// Return value is not guaranteed to be zero-terminated. If the line does not correspond to a function,
// returns `NULL` and sets `name_len` to `(size_t) -1`.
const char * lua_module_function_lines_get(const lua_module_function_lines * module, size_t line, size_t * name_len);

// Send the object to this function to be deallocated.
void lua_module_function_lines_free(lua_module_function_lines * module);

#ifdef __cplusplus
}
#endif

#endif
