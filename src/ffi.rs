use std::{os::raw::c_char, ffi::CStr};

use crate::Module;

#[no_mangle]
unsafe extern "C" fn new_lua_module_function_lines(code: *const c_char) -> *const Module {
    unsafe fn inner(code: *const c_char) -> Option<*const Module> {
        let code = CStr::from_ptr(code).to_str().ok()?;
        let module = Module::new(code).map(Box::new);
        module.map(|b| Box::into_raw(b) as _)
    }
    match inner(code) {
        Some(ptr) => ptr,
        None => std::ptr::null(),
    }
}

#[no_mangle]
unsafe extern "C" fn get_lua_module_function_lines(module: *mut Module, line: usize, name_len: *mut usize) -> *const c_char {
    let module = Box::from_raw(module);
    match module.get_function(line) {
        Some(name) => {
            *name_len = name.len();
            name.as_ptr() as _
        },
        None => std::ptr::null(),
    }

}

#[no_mangle]
unsafe extern "C" fn free_lua_module_function_lines(module: *mut Module) {
    let module = Box::from_raw(module);
    drop(module)
}
