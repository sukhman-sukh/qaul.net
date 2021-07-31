//! # C-API for the threaded libqaul
//! 
//! This is the C compatible FFI of libqaul.
//! It can be used to start libqaul in an own
//! thread and communicate thread safe with it.
//! All functions are thread safe and can be
//! called from any external thread.


/// start libqaul in an own thread
/// 
/// This function initializes and starts libqaul.
/// It needs to be called before any other function
/// of this API.
#[no_mangle]
pub extern "C" fn start() {
    super::start();
}

/// send RPC messages to libqaul
/// 
/// returns 0 on success and negative numbers on failure
/// 
/// 0  : success
/// -1 : pointer is null
/// -2 : message is too big
#[no_mangle]
pub extern "C" fn send_rpc_to_libqaul(message: *const libc::c_uchar, message_length: u32) -> i32 {
    // message-pointer sanity check
    if message.is_null() {
        log::error!("message pointer is null");
        return -1
    }

    // check for message length
    // set a maximum message size to 500'000 bytes
    if message_length > 500000 {
        log::error!("message size to big! size is {} bytes", message_length);
        return -2
    }

    // copy input buffer to libqaul
    let message_length_usize: usize = message_length as usize;
    let mut rust_buffer: Vec<u8> = Vec::with_capacity(message_length_usize);
    unsafe {
        std::ptr::copy_nonoverlapping(message, rust_buffer.as_mut_ptr(), message_length_usize);
    }

    // send it further to libqaul
    super::send_rpc_to_libqaul(rust_buffer);

    // return success
    0
}


/// receive RPC messages from libqaul
/// 
/// You need to provide the pointer to a buffer and declare
/// the length of a buffer.
/// If a message was received, this function copies the message
/// into the buffer.
/// 
/// The function returns the length of the message.
/// The return value '0' means no message was received. 
/// 
/// A negative value is an error.
/// -1 : an error occured
/// -2 : buffer to small
/// -3 : buffer pointer is null
#[no_mangle]
pub extern "C" fn receive_rpc_from_libqaul(buffer: *mut libc::c_uchar, buffer_length: u32) -> i32 {
    // poll rpc channel
    let received = super::receive_rpc_from_libqaul();

    match received {
        Ok(message) => {
            // check if no message
            if message.len() == 0 {
                return 0
            }

            // buffer-pointer sanity check
            if buffer.is_null() {
                log::error!("provided buffer pointer is null");
                return -3
            }

            // check buffer len
            let buffer_length_usize: usize = buffer_length as usize;
            if message.len() >= buffer_length_usize {
                log::error!("Buffer size to small! message size: {} < buffer size {}", message.len(), buffer_length);
                // return -2: buffer size to small
                return -2            
            }

            // copy message into provided buffer
            unsafe {
                std::ptr::copy_nonoverlapping(message.as_ptr(), buffer, message.len());
            }
                
            // // https://doc.rust-lang.org/std/mem/fn.transmute.html
            // let u8_slice = unsafe {
            // &*( &slice as *const [c_char] as *const [u8])
            // };

            // unsafe {
            //     //buffer.copy_from_nonoverlapping(message, message.len());
            //     //let msg = message;
            //     //std::ptr::copy_nonoverlapping(message, buffer, message.len());
            // }

            // return message length
            let len: i32 = message.len() as i32;
            len
        },
        Err(err) => {
            // log error message
            log::error!("{:?}", err);
            // return -1: an error occurred
            -1
        },
    }
}