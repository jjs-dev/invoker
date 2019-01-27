use execute;
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    mem,
    os::raw::c_char,
    time,
};

pub struct Backend(Box<dyn execute::Backend>);

#[no_mangle]
pub unsafe extern "C" fn minion_setup() -> *mut Backend {
    let backend = execute::setup();
    let backend = Box::into_raw(backend);
    backend as *mut Backend
}

#[no_mangle]
pub unsafe extern "C" fn minion_backend_free(b: *mut Backend) {
    let b = Box::from_raw(b);
    mem::drop(b);
}
pub struct DominionOptionsWrapper(execute::DominionOptions);

pub struct DominionWrapper(execute::DominionRef);

#[no_mangle]
pub unsafe extern "C" fn minion_dominion_options_create() -> *mut DominionOptionsWrapper {
    let opts = DominionOptionsWrapper(execute::DominionOptions {
        allow_network: false,
        allow_file_io: false,
        max_alive_process_count: 0,
        memory_limit: 0,
        time_limit: time::Duration::new(0, 0),
        isolation_root: "".into(),
        exposed_paths: vec![],
    });
    let opts = Box::new(opts);
    let opts = Box::into_raw(opts);
    opts
}

#[no_mangle]
pub unsafe extern "C" fn minion_dominion_options_time_limit(
    options: *mut DominionOptionsWrapper,
    seconds: u32,
    nanoseconds: u32,
) {
    (*options).0.time_limit = time::Duration::new(u64::from(seconds), nanoseconds)
}

#[no_mangle]
pub unsafe extern "C" fn minion_dominion_options_process_limit(
    options: *mut DominionOptionsWrapper,
    limit: u32,
) {
    (*options).0.max_alive_process_count = limit as usize
}

#[no_mangle]
pub unsafe extern "C" fn minion_dominion_options_isolation_root(
    options: *mut DominionOptionsWrapper,
    path: *const c_char,
) {
    let s = CStr::from_ptr(path).to_str().unwrap().to_string();
    (*options).0.isolation_root = s;
}

#[no_mangle]
pub unsafe extern "C" fn minion_backend_setopt(
    backend: *mut Backend,
    option_name: *const c_char,
    option_value: *const c_char,
) {
    //TODO
}

#[no_mangle]
pub unsafe extern "C" fn minion_backend_getopt(
    backend: *mut Backend,
    option_name: *const c_char,
    option_value: *mut c_char,
    option_value_size: *mut u32,
) {
    //TODO
}

#[no_mangle]
pub unsafe extern "C" fn minion_dominion_create(
    backend: *mut Backend,
    options: *mut DominionOptionsWrapper,
) -> *mut DominionWrapper {
    let d = (&*backend).0.new_dominion((*options).0.clone()).unwrap();
    let dw = DominionWrapper(d);
    Box::into_raw(Box::new(dw))
}

#[no_mangle]
pub unsafe extern "C" fn minion_dominion_clone(
    d_in: *mut DominionWrapper,
    out1: *mut *mut DominionWrapper,
    out2: *mut *mut DominionWrapper,
) {
    let dw = Box::from_raw(d_in);
    let dw = (*dw).0;
    let dref1 = dw.clone();
    let dref2 = dw;
    *out1 = Box::into_raw(Box::new(DominionWrapper(dref1)));
    *out2 = Box::into_raw(Box::new(DominionWrapper(dref2)));
}

pub struct ChildProcessOptionsWrapper(execute::ChildProcessOptions);
#[no_mangle]
pub unsafe extern "C" fn minion_cp_options_create(
    dominion: *mut DominionWrapper,
) -> *mut ChildProcessOptionsWrapper {
    let dominion = (*Box::from_raw(dominion)).0;
    let opts = ChildProcessOptionsWrapper(execute::ChildProcessOptions {
        path: "".to_string(),
        arguments: vec![],
        environment: HashMap::new(),
        dominion,
        stdio: execute::StdioSpecification {
            stdin: execute::InputSpecification::Null,
            stdout: execute::OutputSpecification::Null,
            stderr: execute::OutputSpecification::Null,
        },
        pwd: "".to_string(),
    });
    Box::into_raw(Box::new(opts))
}

#[no_mangle]
pub unsafe extern "C" fn minion_cp_options_set_image_path(
    options: *mut ChildProcessOptionsWrapper,
    path: *const c_char,
) {
    (*options).0.path = CStr::from_ptr(path).to_str().unwrap().to_string();
}

#[no_mangle]
pub unsafe extern "C" fn minion_cp_options_add_arg(
    options: *mut ChildProcessOptionsWrapper,
    arg: *const c_char,
) {
    let arg = CStr::from_ptr(arg).to_str().unwrap().to_string();
    (*options).0.arguments.push(arg);
}

#[no_mangle]
pub unsafe extern "C" fn minion_cp_options_add_env(
    options: *mut ChildProcessOptionsWrapper,
    var_name: *const c_char,
    var_value: *const c_char,
) {
    let var_name = CStr::from_ptr(var_name).to_str().unwrap().to_string();
    let var_value = CStr::from_ptr(var_value).to_str().unwrap().to_string();
    (*options).0.environment.insert(var_name, var_value);
}

#[repr(u8)]
pub enum StdioMember {
    Stdin,
    Stdout,
    Stderr,
}

#[no_mangle]
pub unsafe extern "C" fn minion_cp_options_set_stdio_handle(
    options: *mut ChildProcessOptionsWrapper,
    member: StdioMember,
    handle: u64,
) {
    let hw = execute::HandleWrapper::new(handle);
    match member {
        StdioMember::Stdin => (*options).0.stdio.stdin = execute::InputSpecification::RawHandle(hw),
        StdioMember::Stdout => {
            (*options).0.stdio.stdout = execute::OutputSpecification::RawHandle(hw)
        }
        StdioMember::Stderr => {
            (*options).0.stdio.stderr = execute::OutputSpecification::RawHandle(hw)
        }
    }
}

pub struct ChildProcessWrapper(Box<dyn execute::ChildProcess>);

#[no_mangle]
pub unsafe extern "C" fn minion_cp_spawn(
    backend: *mut Backend,
    options: *mut ChildProcessOptionsWrapper,
) -> *mut ChildProcessWrapper {
    use std::clone::Clone;
    let cp = (*backend).0.spawn((*options).0.clone()).unwrap();
    let cp = ChildProcessWrapper(cp);
    let cp = Box::new(cp);
    Box::into_raw(cp)
}