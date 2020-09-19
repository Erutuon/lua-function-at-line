#include <stdio.h>
#include <string.h>
#include <limits.h>
#include <stdbool.h>

#include "lua_function_at_line.h"

char * read_file(const char * const path, size_t * len) {
    FILE * file = fopen(path, "rb");
    if (!file) {
        fprintf(stderr, "could not open Lua file\n");
        return NULL;
    }
    fseek(file, 0, SEEK_END);
    long length = ftell(file);
    if (len != NULL) *len = length;
    fseek(file, 0, SEEK_SET);
    char * buffer = malloc(length + 1);
    if (buffer) {
        fread(buffer, 1, length, file);
        buffer[length] = '\0';
    }
    fclose(file);
    return buffer;
}

int main(int argc, char * * argv) {
    if (argc < 2) {
        fprintf(stderr, "expected Lua file name\n");
        return 1;
    }
    const char * lua_path = argv[1];
    size_t len = 0;
    char * lua_code = read_file(lua_path, &len);
    if (!lua_code) {
        printf("failed to read file\n");
        return 1;
    }
    Module * module = lua_module_function_lines_new(lua_code);
    if (!module) {
        printf("failed to parse Lua code:\n%s\n", lua_code);
        goto free_code;
    }
    size_t function_len = -1;
    size_t line = 1;
    const char * lua_code_end = lua_code + len;
    // Continue incrementing `line` while `memchr` finds a newline.
    const char * line_start = lua_code;
    while (line_start != NULL && line_start < lua_code_end) {
        const char * function = lua_module_function_lines_get(module, line, &function_len);
        const char * next_line_break = memchr(line_start, '\n', lua_code_end - line_start);
        size_t line_len = (next_line_break != NULL) ? next_line_break - line_start : lua_code_end - line_start;
        if (function_len != -1) {
            if (function_len > (size_t) INT_MAX) {
                printf("%zu: %.*s ...", line, INT_MAX, function);
            } else {
                printf("%zu: %.*s", line, (int) function_len, function);
            }
        } else {
            printf("%zu: <unknown>", line);
        }
        if (line_len > 0) {
            if (line_len > (size_t) INT_MAX) {
                printf("%.*s ...", INT_MAX, line_start);
            } else {
                printf("%.*s", (int) line_len, line_start);
            }
        }
        printf("\n");
        if (next_line_break == NULL) break;
        line_start = next_line_break + 1, ++line;
    }
    lua_module_function_lines_free(module);
    free_code:
    free(lua_code);
    return 0;
}
