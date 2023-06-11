mod tdjson;
mod authentication;
mod error;

use std::collections::HashMap;
use std::ffi::{c_char, c_double, c_int, c_void, CStr, CString};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::cell::{Cell, RefCell};
use std::{ffi, fs, io};
use std::fmt::format;
use std::fs::File;
use std::path::{Iter, Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, mpsc, Mutex};
use std::task::{Context, Poll};
use std::time::SystemTime;

use serde_json::{Value, json};

use authentication as auth;

use tdjson::*;
use error::*;

#[derive(Debug)]
pub struct TDApp {
    client_id: i32,
    current_query_id: AtomicU64,
    error_log_file: Mutex<File>,
}

impl TDApp {

    pub fn create() -> Self {

        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let _ = fs::create_dir_all("./logs").unwrap();
        let log_file = File::create(format!("./logs/error_{}.log", time)).unwrap();

        TDApp {
            client_id: unsafe { td_create_client_id() },
            current_query_id: AtomicU64::new(0),
            error_log_file: Mutex::new(log_file),
        }
    }

    pub fn execute_query(query: &str) -> Result<Option<String>, std::ffi::NulError> {

        unsafe {
            let request_cstring = CString::new(query)?;

            let opt_response_str = td_execute(request_cstring.as_ptr())
                .as_ref()
                .map(
                    |chars| CStr::from_ptr(chars)
                        .to_string_lossy()
                        .into_owned()
                );

            return Ok(opt_response_str)
        }
    }

    pub fn receive(&self, timeout: f64) -> Option<String> {
        unsafe {
            let response = td_receive(timeout);

            return response.as_ref().map(
                |chars| CStr::from_ptr(chars)
                    .to_string_lossy()
                    .into_owned()
            )
        }
    }

    pub fn send_query(&self, query: &str) -> Result<(), std::ffi::NulError> {
        unsafe {
            td_send(
                self.client_id,
                CString::new(query)?.as_ptr()
            );
        }

        self.current_query_id.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    fn next_update_json(&self) -> Option<Value> {
        if let Some(response) = self.receive(1.0) {

            let json = serde_json::from_str::<Value>(&response)
                .expect("TDLib прислал невалидный json");

            if json["@type"] == "error" {
                self.error_handling(&json).unwrap();
            }

            return Some(json)
        }

        None
    }

    pub fn skip_all_update(&self, timeout: f64) {
        while let Some(update) = self.receive(timeout) {
            println!("Update ==> {update}");
        }
    }

    pub fn account_auth(&self) -> Result<(), TDAppError> {

        println!("Авторизация...");
        io::stdout().flush()?;

        TDApp::execute_query(&json!({
            "@type": "setOption",
            "name": "ignore_background_updates",
            "value": "true"
        }).to_string()).expect("Строка содержала нулебой байт");

        self.send_query(
            &authentication::get_tdlib_params_request(None)
        ).expect("Строка содержала нулебой байт");

        let mut are_authorized = false;

        while !are_authorized {
            if let Some(response) = self.receive(1.0) {

                let json = serde_json::from_str::<Value>(&response)
                    .expect("TDLib прислал невалидный json");

                if json["@type"] == "error" {
                    self.error_handling(&json)?;
                }


                let json_type = json["@type"]
                    .as_str()
                    .expect("TDLib прислал невалидный json");

                if json_type != "updateAuthorizationState" {

                    io::stdout().write_all(format!("Update ===> {}\n----\n", json).as_bytes())?;
                    io::stdout().flush()?;
                    continue

                }
                println!("AuthUpdate ===> {}\n----\n", json);
                io::stdout().flush()?;

                let authorization_state = json["authorization_state"]["@type"]
                    .as_str()
                    .expect("TDLib прислал невалидный json");

                match authorization_state {

                    "authorizationStateWaitTdlibParameters" => {
                        println!("Sending TdlibParameters");
                        io::stdout().flush()?;

                        self.send_query(
                            &auth::get_tdlib_params_request(None)
                        ).expect("Строка содержала нулебой байт");

                    },

                    "authorizationStateWaitPhoneNumber" =>
                        self.send_query(&auth::get_phone_number_request()).unwrap(),

                    "authorizationStateWaitCode" =>
                        self.send_query(&auth::get_check_code_request()).unwrap(),

                    "authorizationStateWaitPassword" =>
                        self.send_query(&auth::get_check_password_request()).unwrap(),

                    "authorizationStateReady" => {
                        println!("|==|==|==> Authorization is completed <==|==|==|");
                        are_authorized = true;
                        continue;
                    },

                    "authorizationStateClosed" => println!("Обновление статуса авторизации: {}", json),
                    "authorizationStateClosing" => println!("Обновление статуса авторизации: {}", json),
                    "authorizationStateLoggingOut" => {
                        println!("|==|==|==> Logging out <==|==|==|")
                    },
                    "authorizationStateWaitEncryptionKey" => println!("Обновление статуса авторизации: {}", json),
                    "authorizationStateWaitOtherDeviceConfirmation" => println!("Обновление статуса авторизации: {}", json),
                    "authorizationStateWaitRegistration" => println!("Обновление статуса авторизации: {}", json),
                    _ => println!("Другие обновления авторизации: {}", authorization_state)
                }
            }
        }

        Ok(())
    }

    fn error_handling(&self, json: &Value) -> Result<(), TDAppError> {

        println!("----------------------------------\nFOUND ERROR: {}\n", json);

        let error_message = json["message"].as_str().expect("TDLib прислал невалидный json");

        match error_message {
            "PHONE_NUMBER_INVALID" =>
                self.send_query(&auth::get_phone_number_request()).unwrap(),
            "PHONE_CODE_INVALID" =>
                self.send_query(&auth::get_check_code_request()).unwrap(),
            "PASSWORD_HASH_INVALID" =>
                self.send_query(&auth::get_check_password_request()).unwrap(),
            _ => println!("Unsupported telegram error")
        }

        let mut log_file = self.error_log_file.lock().unwrap();

        log_file.write_all(
            format!("\nError ==> {json}\n|").as_bytes()
        )?;

        return Ok(())
    }

    pub fn get_me(&self) -> Value {
        self.send_query(&json!({
            "@type": "getMe"
        }).to_string()).unwrap();

        while let Some(json_response) = self.next_update_json() {
            if json_response["@type"].as_str().unwrap() == "user" {
                return json_response
            }
        }
        Value::Null
    }

    pub fn create_chat(&self) -> Value {

        let my_acc = self.get_me();

        self.send_query(&dbg!(json!({
            "@type": "createNewSupergroupChat",
            "title": "TelegramDrive"
        }).to_string())).unwrap();

        loop {
            if let Some(response) = self.receive(1.0) {

                let json = serde_json::from_str::<Value>(&response)
                    .expect("TDLib прислал невалидный json");

                if json["@type"] == "error" { self.error_handling(&json).unwrap(); }

                if json["@type"].as_str().unwrap() == "chat" {
                    return json;
                }
            }
        }
    }

    pub fn get_chat(&self, chat_id: i64) -> Value {
        self.send_query(&json!({
            "@type": "getChat",
            "chat_id": chat_id
        }).to_string()).unwrap();

        loop {
            if let Some(response) = self.receive(1.0) {

                let json = serde_json::from_str::<Value>(&response)
                    .expect("TDLib прислал невалидный json");

                if json["@type"] == "error" {
                    self.error_handling(&json).unwrap();
                }

                if json["@type"] == "chat" && json["id"].as_i64().unwrap() == chat_id {
                    println!("chat: {}\n", json);
                    return json
                } else {
                    println!("Update: {}\n", json);
                }
            }
        }
    }

    pub fn load_all_messages(&self, chat_id: i64) -> Vec<Value> {
        self.send_query(&json!({
            "@type": "getChatHistory",
            "chat_id": chat_id,
            "limit": 20
        }).to_string()).unwrap();

        let mut chat_all_messages = vec![];

        while let Some(json_update) = self.next_update_json() {

            if json_update["@type"] == "messages" {

                let total_count = json_update["total_count"].as_u64().unwrap();

                if total_count <= 0 {
                    println!("Сообщений больше нет: {}", json_update);
                    return chat_all_messages
                }

                let messages = json_update["messages"].as_array().unwrap();
                chat_all_messages.extend_from_slice(&messages);

                println!("messages: {}\n", json_update);

                let last_message_id =
                    if let Some(message) = messages.last() {
                        message["id"].as_i64().unwrap()
                    } else {
                        return chat_all_messages
                    };

                self.send_query(&json!({
                        "@type": "getChatHistory",
                        "chat_id": chat_id,
                        "limit": 30,
                        "from_message_id": last_message_id
                    }).to_string()).unwrap();

            } else {
                //println!("Update: {}\n", json);
            }
        }

        vec![]
    }

    pub fn get_message(&self, message_id: i64, chat_id: i64) -> Value {

        self.send_query(&json!({
            "@type": "getMessage",
            "chat_id": chat_id,
            "message_id": message_id
        }).to_string()).unwrap();

        loop {
            if let Some(response) = self.receive(1.0) {

                let json = serde_json::from_str::<Value>(&response)
                    .expect("TDLib прислал невалидный json");

                if json["@type"] == "error" {
                    self.error_handling(&json).unwrap();
                }

                if json["@type"] == "message" {

                    if json["chat_id"] == chat_id && json["id"] == message_id {
                        return json
                    }
                }
            }
        }
    }

    pub fn upload_file(&self, file_path: &Path, chat_id: i64) -> (i64, Value) {

        self.send_query(&json!({
                    "@type": "sendMessage",
                    "chat_id": chat_id,
                    "input_message_content": {
                        "@type": "inputMessageDocument",
                        "document": {
                            "@type": "inputFileLocal",
                            "path": dbg!(file_path.display().to_string())
                        }
                    }
                }).to_string()).unwrap();


        // Возможны проблемы с парсингом json
        // Сделать проверку LocalFile.path == file_path

        println!("Запрос на отправку файла отправлен.");

        while let Some(json_update) = self.next_update_json() {

            match json_update["@type"].as_str().unwrap() {
                "updateFile" => {
                    println!("FileUpdates: {}\n", json_update);
                }
                "updateMessageSendSucceeded" => {

                    //let file_id = json_update["message"]["content"]["document"]["id"].as_i64().unwrap();

                    println!("FULFILE: {}\n", json_update);
                    return (2, json_update)
                }

                _ => {}
            }
        }
        panic!("Не удалось загрузить файл")
    }

    pub fn download_file(&self, file_id: i64) -> Result<Value, ()> {

        self.send_query(&json!({
            "@type": "downloadFile",
            "file_id": file_id,
            "priority": 1
        }).to_string()).unwrap();

        let mut download_file_expected_size = 0;

        while let Some(json_update) = self.next_update_json() {

            match json_update["@type"].as_str().unwrap() {

                "file" => {
                    if json_update["id"] == file_id {
                        download_file_expected_size = json_update["expected_size"].as_u64().unwrap();
                    }
                    if json_update["local"]["is_downloading_completed"].as_bool().unwrap() {
                        return Ok(json_update);
                    }

                    println!("{}", json_update);
                },

                "updateFile" => {
                    println!("FileUpdates: {}\n", json_update);

                    if
                        json_update["file"]["id"] == file_id &&
                        json_update["file"]["local"]["is_downloading_completed"].as_bool().unwrap() &&
                        json_update["file"]["local"]["downloaded_size"] == download_file_expected_size
                    {
                        return Ok(json_update);
                    }
                }
                _ => {}
            }
        }
        Err(())
    }

    pub fn delete_message(&self, chat_id: i64, vec_message_id: &[i64]) -> Result<(), ()> {
        self.send_query(&json!({
            "@type": "deleteMessages",
            "chat_id": chat_id,
            "message_ids": vec_message_id,
            "revoke": true
        }).to_string()).unwrap();

        Ok(())
    }
}
