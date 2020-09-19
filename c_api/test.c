// To compile:
// $ cargo build --release
// $ cp target/release/liblua_function_at_line_c.so .
// $ gcc test.c -o test -L. -Iinclude -llua_function_at_line_c -lm
//
// To run:
// $ ./test some_lua_file.lua

#include <stdio.h>
#include <string.h>
#include <stdbool.h>
#include <math.h>

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

size_t count_lines(const char * str, size_t len) {
    size_t lines = 1;
    const char * end = str + len;
    const char * line_break = str;
    while (line_break < end && (line_break = memchr(line_break, '\n', end - line_break))) ++lines, ++line_break;
    return lines;
}

typedef struct string_ref {
    const char * ptr;
    size_t len;
} string_ref;

string_ref * get_lines(const char * str, size_t len, size_t * out_line_count) {
    size_t line_count = count_lines(str, len);
    string_ref * const lines = malloc(line_count * sizeof (string_ref));
    if (lines == NULL) exit(1);
    if (out_line_count != NULL) *out_line_count = line_count;
    const char * end = str + len;
    const char * line_start = str;
    string_ref * cur_line = lines;
    while (line_start != NULL && line_start < end && cur_line < lines + line_count) {
        const char * next_line_break = memchr(line_start, '\n', end - line_start);
        size_t line_len = (next_line_break != NULL) ? next_line_break - line_start : end - line_start;
        // Strip CR in CRLF.
        if (next_line_break != NULL && next_line_break - 1 >= str && *(next_line_break - 1) == '\r') {
            --line_len;
        }
        cur_line->ptr = line_start;
        cur_line->len = line_len;
        if (next_line_break == NULL) break;
        line_start = next_line_break + 1;
        ++cur_line;
    }
    return lines;
}

void get_function_names(lua_module_function_lines * module, string_ref * const names, size_t len) {
    string_ref * cur_name = names;
    // This assumes that lines in full_moon are zero-based.
    size_t line = 0;
    while (line < len) {
        size_t name_len = -1;
        const char * name = lua_module_function_lines_get(module, line, &name_len);
        cur_name->ptr = name;
        cur_name->len = name_len;
        ++cur_name, ++line;
    }
}

void show_lines_with_function_names(const char * lua_code, size_t lua_code_len) {
    lua_module_function_lines * module = lua_module_function_lines_new(lua_code);
    if (!module) {
        printf("failed to parse Lua code:\n%s\n", lua_code);
        return;
    }
    size_t line_count = 0;
    string_ref * const lines = get_lines(lua_code, lua_code_len, &line_count);
    string_ref * const function_names = malloc(line_count * sizeof (string_ref));
    get_function_names(module, function_names, line_count);
    if (function_names == NULL) goto free_all;

    // Get longest function name for `printf` format string.
    size_t max_function_name_len = 0;
    for (const string_ref * name = function_names; name < function_names + line_count; ++name) {
        if (name->len != (size_t) -1) {
            max_function_name_len = (name->len > max_function_name_len) ? name->len : max_function_name_len;
        }
    }

    // Get largest number of decimal digits in line count for `printf` format string.
    size_t line_number_len = (size_t) log10((double) line_count) + 1;

    size_t line_number = 1;
    const string_ref * cur_line = lines;
    for (string_ref * name = function_names; name < function_names + line_count; ++line_number, ++cur_line, ++name) {
        const char * function_name = name->ptr;
        size_t function_name_len = name->len;
        if (function_name == NULL) {
            function_name = "<unknown>";
            function_name_len = sizeof "<unknown>" - 1;
        }
        // Assume cur_line-> len is less than or equal to INT_MAX.
        printf(
            "%*zu   %*s%.*s   %.*s\n",
            (int) line_number_len, line_number,
            (int) (max_function_name_len - function_name_len), "",
            (int) function_name_len, function_name,
            (int) cur_line->len, cur_line->ptr
        );
    }
    free_all:
    lua_module_function_lines_free(module);
    free(function_names);
    free(lines);
}

int main(int argc, char * * argv) {
    if (argc < 2) {
        fprintf(stderr, "expected Lua file name\n");
        return 1;
    }
    const char * const lua_path = argv[1];
    size_t lua_code_len = 0;
    char * const lua_code = read_file(lua_path, &lua_code_len);
    if (!lua_code) {
        printf("failed to read file\n");
        return 1;
    }
    show_lines_with_function_names(lua_code, lua_code_len);
    free(lua_code);
    return 0;
}
