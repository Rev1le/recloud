use std::ffi::{c_char, c_double, c_int, c_void};

#[link(name = "tdjson")]
extern "C" {
    /// Returns an opaque identifier of a new TDLib instance.
    /// The TDLib instance will not send updates until the first request is sent to it.
    pub fn td_create_client_id() -> c_int;

    /// Sends request to the TDLib client. May be called from any thread.
    pub fn td_send(client_id: c_int, request: *const c_char) -> c_void;

    /// Receives incoming updates and request responses.
    /// Must not be called simultaneously from two different threads.
    /// The returned pointer can be used until the next call to td_receive or td_execute,
    /// after which it will be deallocated by TDLib.
    pub fn td_receive(timeout: c_double) -> *const c_char;

    /// Synchronously executes a TDLib request.
    /// A request can be executed synchronously,
    /// only if it is documented with "Can be called synchronously".
    /// The returned pointer can be used until the next call to td_receive or td_execute,
    /// after which it will be deallocated by TDLib.
    pub fn td_execute(request: *const c_char) -> *const c_char;

    /// Sets the callback that will be called when a message is added to the internal TDLib log.
    /// None of the TDLib methods can be called from the callback.
    /// By default the callback is not set.
    pub fn td_set_log_message_callback(
        max_verbosity_level: c_int,
        callback: extern "C" fn(c_int, *const c_char) -> c_void
    ) -> c_void;

    /// Creates a new instance of TDLib.
    #[allow(dead_code)]
    pub fn td_json_client_create() -> *const c_void;

    /// Sends request to the TDLib client. May be called from any thread.
    #[allow(dead_code)]
    pub fn td_json_client_send(client: *const c_void, request: *const c_char) -> c_void;

    /// Receives incoming updates and request responses from the TDLib client.
    /// May be called from any thread,
    /// but must not be called simultaneously from two different threads.
    /// Returned pointer will be deallocated by TDLib during next call to td_json_client_receive
    /// or td_json_client_execute in the same thread, so it can't be used after that.
    #[allow(dead_code)]
    pub fn td_json_client_receive(client: *const c_void, timeout: c_double) -> *const c_char;

    /// Synchronously executes TDLib request. May be called from any thread.
    /// Only a few requests can be executed synchronously.
    /// Returned pointer will be deallocated by TDLib during next call to td_json_client_receive
    /// or td_json_client_execute in the same thread, so it can't be used after that.
    #[allow(dead_code)]
    pub fn td_json_client_execute(client: *const c_void, request: *const c_char) -> *const c_char;

    /// Destroys the TDLib client instance.
    /// After this is called the client instance must not be used anymore.
    #[allow(dead_code)]
    pub fn td_json_client_destroy(client: *const c_void) -> c_void;
}

fn td_log_message_callback_ptr(verbosity_level: c_int, message: *const c_char) -> () {
    println!("{:?} - {:?}", verbosity_level, message);
}