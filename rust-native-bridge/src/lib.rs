use std::os::raw::{c_char};
use std::ffi::{CString, CStr};
use bytes::{BytesMut, BufMut};
use serde::{Deserialize, Serialize};
use serde_json::{Value, Result};
use url::{Url, Host, Position};
use serde_urlencoded;
use form_urlencoded;
use std::collections::HashMap;
use futures::executor::block_on;
use futures::Future;
use reqwest;
use bytes::Bytes;
use lazy_static::lazy_static;

#[cfg(test)]
mod tests {
    // 注意这个惯用法：在 tests 模块中，从外部作用域导入所有名字。
    use super::*;
    #[test]
    fn test_swapi() {
        swapi_call_with_barrier();
    }
}

mod network_interface;

#[macro_use]
extern crate anyhow;  //an `extern crate` loading macros must be at the crate root
//thread sleep
use std::{thread, time};

//measure time
use std::time::Instant;
//DTO
use network_interface::{swapi_callback};
//for thread barrier
use std::sync::{Arc, Mutex, Condvar};





#[no_mangle]
pub extern fn test(to: *const c_char) -> *mut c_char {
    println!("Main started");

    swapi_call_with_barrier();
    println!("Main finished");

    // swapi_call_with_thread_sleep();
    CString::new("swapi_call_with_barrier end ".to_owned()).unwrap().into_raw()
}

#[no_mangle]
pub extern fn rust_greeting_free(s: *mut c_char) {
    unsafe {
        if s.is_null() { return }
        CString::from_raw(s)
    };
}



//Create callback
struct Callback {
    start: Instant,
    unlock: Box<dyn FnMut()>,
}

impl Callback {
    fn new(unlock: Box<dyn FnMut()>) -> Callback {
        Callback {
            start: Instant::now(),
            unlock,
        }
    }
}

//Send - types that can be transferred across thread boundaries.
unsafe impl Send for Callback {}

//require to share it between threads
impl swapi_callback for Callback {

    fn onload(&mut self, s: &str) {
        let diff = self.start.elapsed().as_millis();
        println!("Request:{}", s);
        //notify lock that thread finished work
        // (self.unlock)();
    }
    fn on_error(&mut self, s: &str) {
        println!("Error: {:#?}", s);
        //notify lock that thread finished work
        (self.unlock)();
    }
}
fn swapi_call_with_barrier() {

    //barrier
    let con_var = Arc::new((Mutex::new(false), Condvar::new()));
    //Will be used in another thread
    let con_var_send = con_var.clone();

    //Callback that will nlock thread
    let unlock = move || {
        let (lock, cvar) = &*con_var_send;
        let mut started = lock.lock().unwrap();
        *started = true;
        // We notify the condvar that the value has changed.
        cvar.notify_one();
    };

    //call swapi client
    let client = network_interface::SwapiClient::new();

    let url = "https://mapihmtft.staging.gamepanda.tw/phone/switch/getstate";
    let ps = "uuid=F9A70247-A359-46F9-8F41-747F0D80FEEB&pl=appstore&os=ios&gn=hmtft&packversion=1.0.0&systemversion=13.3&time=1626260019&sign=37e01b9a24c2a908f89d69a6b8ac44fb";
    client.do_post_request(url,ps,Box::new(Callback::new(Box::new(unlock))));

    //wait for thread to finish
    // Wait for the thread to start up.
    let (lock, cvar) = &*con_var;
    let mut started = lock.lock().unwrap();
    while !*started {
        started = cvar.wait(started).unwrap();
    }

}



