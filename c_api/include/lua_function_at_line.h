#ifndef LUA_FUNCTION_AT_LINE_H
#define LUA_FUNCTION_AT_LINE_H

#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Module Module;

Module * lua_module_function_lines_new(const char * code);

const char * lua_module_function_lines_get(const Module * module, size_t line, size_t * name_len);

void lua_module_function_lines_free(Module * module);

#ifdef __cplusplus
}
#endif

#endif
