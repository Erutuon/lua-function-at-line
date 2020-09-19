#ifndef LUA_FUNCTION_AT_LINE_H
#define LUA_FUNCTION_AT_LINE_H

#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct lua_module_function_lines lua_module_function_lines;

lua_module_function_lines * lua_module_function_lines_new(const char * code);

const char * lua_module_function_lines_get(const lua_module_function_lines * module, size_t line, size_t * name_len);

void lua_module_function_lines_free(lua_module_function_lines * module);

#ifdef __cplusplus
}
#endif

#endif
