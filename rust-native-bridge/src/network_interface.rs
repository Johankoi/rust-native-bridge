use std::result;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use futures::SinkExt;
use std::collections::HashMap;
use url::{Url};
use form_urlencoded;
use std::os::raw::{c_char, c_void};
use std::ops::Deref;
use std::sync::Arc;
use std::ffi::CString;
use std::ffi::CStr;
use std::str;
use rayon::scope;
// https://pkolaczk.github.io/multiple-threadpools-rust/
lazy_static! {
static ref RUN_TIME: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .unwrap();
}

pub trait swapi_callback {
    fn onload(&mut self, s: &str);
    fn on_error(&mut self, s: &str);
}

//Local callback for loading
pub struct Callback {
    pub result: Box<dyn FnMut(String)>,
    pub error: Box<dyn FnMut(String)>,
}
unsafe impl Send for Callback {}

#[allow(non_snake_case)]
impl swapi_callback for Callback {
    fn onload(&mut self, s: &str) {
        (self.result)(s.to_string());
    }

    fn on_error(&mut self, s: &str) {
        (self.error)(s.to_string());
    }
}


//you need reference to owner context to return data
#[repr(C)]
pub struct NetReqCallback {
    owner: *mut c_void,
    onResult: extern fn(owner: *mut c_void, arg: *const c_char),
    onError: extern fn(owner: *mut c_void, arg: *const c_char),
}
impl Copy for NetReqCallback {}
unsafe impl Send for NetReqCallback {}

impl Deref for NetReqCallback {
    type Target = NetReqCallback;
    fn deref(&self) -> &NetReqCallback {
        &self
    }
}
impl Clone for NetReqCallback {
    fn clone(&self) -> Self {
        *self
    }
}


#[repr(C)]
pub struct SwapiClient();

impl SwapiClient {
    pub fn new() -> SwapiClient {
        SwapiClient()
    }

    pub fn do_post_request(&self, url: &str,
                           parm_str: &str,
                           mut callback: Box<dyn swapi_callback + Send>) {
        rayon::scope(|s| {
            s.spawn(async move {
                let res = SwapiClient::do_request(url,parm_str).await;
                match res {
                    Ok(r) => { callback.onload(r.as_str()); }
                    Err(e) => { callback.on_error(format!(" request err:{} ",e).as_str()); }
                }
            });
        });

    }

    pub async fn do_request(url: &str, parm_str: &str) -> result::Result<String, Error> {
        let client = reqwest::Client::new();
        let parse_form_url = form_urlencoded::parse(parm_str.as_bytes());
        let hash_query: HashMap<_, _> = parse_form_url.into_owned().collect();
        let resp = client.post(url)
            .form(&hash_query)
            .send()
            .await
            .map_err(|e| anyhow!("request {:?}", e))?;

        let r = resp
            .text()
            .await
            .map_err(|e| anyhow!("parse {:?}", e))?;
        return Ok(r);
    }
}


//Create client
#[no_mangle]
pub extern "C" fn create_swapi_client() -> *mut SwapiClient {
    Box::into_raw(Box::new(SwapiClient::new()))
}

//Release memory
#[no_mangle]
pub unsafe extern "C" fn free_swapi_client(client: *mut SwapiClient) {
    assert!(!client.is_null());
    Box::from_raw(client);
}



#[no_mangle]
pub unsafe extern "C" fn http_request_post(client: *mut SwapiClient, url: *const c_char, parm_str: *const c_char, callback: NetReqCallback)
{
    assert!(!client.is_null());
    let local_client = client.as_ref().unwrap();
    let cb = Callback {
        result: Box::new(move |result| {
            let result_str = CString::new(result.to_owned()).unwrap().into_raw();
            (callback.onResult)(callback.owner, result_str);
        }),
        error: Box::new(move |error| {
            let error_message = CString::new(error.to_owned()).unwrap().into_raw();
            (callback.onError)(callback.owner, error_message);
        }),
    };

    let c_str_url: &CStr = unsafe { CStr::from_ptr(url) };
    let  url_str: &str = c_str_url.to_str().unwrap();
    let c_str_parm: &CStr = unsafe { CStr::from_ptr(parm_str) };
    let str_parm: &str = c_str_parm.to_str().unwrap();

    let local_client = client.as_ref().unwrap();
    local_client.do_post_request(url_str,str_parm,Box::new(cb));

}






