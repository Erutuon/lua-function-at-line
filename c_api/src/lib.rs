use std::{os::raw::c_char, ffi::CStr};

use lua_function_at_line::Module;

#[no_mangle]
pub unsafe extern "C" fn lua_module_function_lines_new(code: *const c_char) -> *mut Module {
    unsafe fn inner(code: *const c_char) -> Option<*mut Module> {
        let code = CStr::from_ptr(code).to_str().ok()?;
        Module::new(code).map(|m| Box::into_raw(Box::new(m)))
    }
    match inner(code) {
        Some(ptr) => ptr,
        None => std::ptr::null::<Module>() as _,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lua_module_function_lines_get(module: *const Module, line: usize, name_len: *mut usize) -> *const c_char {
    let module = &*module;
    match module.get_function(line) {
        Some(name) if name.len() < !0 => {
            if name_len != std::ptr::null::<usize>() as _ {
                *name_len = name.len() as _;
            }
            name.as_ptr() as _
        },
        _ => {
            if name_len != std::ptr::null::<usize>() as _ {
                *name_len = !0;
            }
            std::ptr::null()
        },
    }
}

#[no_mangle]
pub unsafe extern "C" fn lua_module_function_lines_free(module: *mut Module) {
    drop(Box::from_raw(module));
}
