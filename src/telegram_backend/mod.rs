
// use std::{
//     thread::JoinHandle,
//     collections::HashMap,
//     fs::{self, File},
//     path::PathBuf,
//     sync::{Arc, mpsc, Mutex,
//            atomic::{AtomicBool, Ordering}
//     }
// };
// use std::cell::RefCell;
// use std::io::{Read, Write};
// use std::path::Path;
// use std::sync::atomic::AtomicI64;
// use tokio::sync::RwLock as AsyncRwLock;
// use async_trait;
// use serde::de::Unexpected::Str;
//
// use serde_json::{json, Value};
// use telegram_drive_core::{self, TDApp};
// use telegram_drive_file::file_separation;
//
// use crate::cloud::CloudError;
// use crate::cloud_backend_traits::AsyncCloudBackend;
// use crate::r#mod::VFSFile;

//const CLOUD_CHAT_ID: i64 = -1001976761155;

//type CallbackEvent = Box<dyn FnOnce(&TDApp) -> bool + Send + 'static>;

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use serde_json::{Value, json};
use crate::cloud::error::CloudError;
use crate::CloudBackend;
use crate::core::TDApp;

// pub struct IO {
//     input: impl Read,
//     output: impl Write
// }

#[derive(Debug)]
pub struct TelegramBackend {
    telegram_app: TDApp,
    cloud_chat_id: i64,
    cloud_chat: Value,
    files: RwLock<HashMap<String, (i64, Value)>>,
}

impl TelegramBackend {

    fn get_cloud_chat_id(app: &TDApp) -> i64 {

        let cloud_chat_id: i64;

        if let Ok(mut f) = File::open("mod.json") {
            let mut file_str = String::new();
            f.read_to_string(&mut file_str).unwrap();

            let json_file = serde_json::from_str::<Value>(&file_str)
                .expect("mod.json содержит невалидный json");

            cloud_chat_id =  json_file["cloud_chat_id"].as_i64().unwrap();

        } else {
            let new_cloud_chat = app.create_chat();

            cloud_chat_id = new_cloud_chat["id"].as_i64().unwrap();

            let json_file = json!({
                "cloud_chat_id": cloud_chat_id,
                "cloud_chat": new_cloud_chat
            }).to_string();

            fs::write("mod.json", json_file.as_bytes()).unwrap();
        }

        return cloud_chat_id;
    }

    fn get_files_from_message(messages: Vec<Value>) -> HashMap<String, (i64, Value)> {

        let mut files_hm = HashMap::with_capacity(messages.len());

        for message in messages {
            let message_id = message["id"].as_i64().unwrap();

            if !message["content"]["document"].is_null() {

                let document = &message["content"]["document"];
                let file_name = document["file_name"].as_str().unwrap().to_owned();
                let file_id = document["document"]["id"].as_i64().unwrap();

                files_hm.insert(file_name.to_owned(), (file_id, message));
            }
        }

        files_hm
    }
}

impl CloudBackend for TelegramBackend {
    fn create(input: impl Read, output: impl Write) -> Self {


        // Отключение вывод логов в консоль
        TDApp::execute_query(&json!({
            "@type": "setLogVerbosityLevel",
            "new_verbosity_level": 0
        }).to_string()).unwrap();

        //panic!("TestRun");

        // Создание телеграм клиента
        let mut app = TDApp::create();
        app.account_auth().expect("Ошибка авторизации в Telegram");
        app.skip_all_update(0.1);

        let cloud_chat_id: i64 = TelegramBackend::get_cloud_chat_id(&app);
        let cloud_chat = dbg!(app.get_chat(cloud_chat_id));
        let messages = dbg!(app.load_all_messages(cloud_chat_id));
        let files = TelegramBackend::get_files_from_message(messages);

        return Self {
            telegram_app: app,
            cloud_chat_id,
            cloud_chat,
            files: RwLock::new(files),
        }
    }

    fn load(&self) -> Result<(), CloudError> {
        todo!()
    }

    fn upload_file(&self, file_path: &Path) -> Result<(), CloudError> {
        todo!()
    }

    fn download_file(&self, file_path: &Path) -> Result<(), CloudError> {
        todo!()
    }

    fn remove_file(&self, file_path: &Path) -> Result<(), CloudError> {
        todo!()
    }

    fn check_file(&self, file_name: &str) -> bool {
        todo!()
    }

    fn close(self) -> Result<(), CloudError> {
        todo!()
    }
}


/*

#[async_trait::async_trait]
impl AsyncCloudBackend for TelegramBackend {
    fn create() -> Self {
        // Отключение вывод логов в консоль
        futures::executor::block_on(TDApp::execute_query(&json!({
            "@type": "setLogVerbosityLevel",
            "new_verbosity_level": 0
        }).to_string())).unwrap();

        // Создание телеграм клиента
        let mut app = futures::executor::block_on(TDApp::create());
        futures::executor::block_on(app.account_auth()).expect("Ошибка авторизации в Telegram");
        futures::executor::block_on(app.skip_all_update(0.1));

        let cloud_chat_id: i64;

        if let Ok(mut f) = File::open("mod.json") {
            let mut file_str = String::new();
            f.read_to_string(&mut file_str).unwrap();

            let json_file = serde_json::from_str::<Value>(&file_str)
                .expect("mod.json содержит невалидный json");

            cloud_chat_id = json_file["cloud_chat_id"].as_i64().unwrap();

        } else {

            let new_cloud_chat = futures::executor::block_on(app.create_chat());

            cloud_chat_id = new_cloud_chat["id"].as_i64().unwrap();

            let json_file = json!({
                "cloud_chat_id": cloud_chat_id,
                "cloud_chat": new_cloud_chat
            }).to_string();

            fs::write("mod.json", json_file.as_bytes()).unwrap();
        }

        let cloud_chat = dbg!(futures::executor::block_on(app.get_chat(cloud_chat_id)));

        let messages = dbg!(futures::executor::block_on(app.load_all_messages(cloud_chat_id)));

        //dbg!(app.get_message(messages[5], CLOUD_CHAT_ID).await);
        //println!("{}", serde_json::to_string(&messages[0..5]).unwrap());

        return Self {
            cloud_chat_id,
            telegram: app,
            cloud_chat,
            files: AsyncRwLock::new(TelegramBackend::get_files_from_message(messages)),
        }
    }
    async fn load_backend(&self) -> Result<(), CloudError> {
        // let res_send_query = self.telegram.borrow_mut().send_query(&json!({
        //     "@type": "ddd",
        //     "chat_id": CLOUD_CHAT_ID
        // }).to_string());

        Ok(())
    }

    async fn upload_file(&self, file_path: &Path) -> Result<(), CloudError> {
        let result_upload = self.telegram.upload_file(file_path, self.cloud_chat_id).await;
        let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();

        self.files.write().await.insert(file_name, result_upload);
        Ok(())
    }

    async fn download_file(&self, file_path: &Path) -> Result<(), CloudError> {
        let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
        let file_id = self.files.read().await.get(&file_name).unwrap().0;

        let download_file = self.telegram.download_file(file_id).await.unwrap();

        Ok(())
    }

    async fn remove_file(&self, file_path: &Path) -> Result<(), CloudError> {
        let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();

        let (file_id, remove_message) = self.files.write().await.remove(&file_name).unwrap();

        let remove_message_id = remove_message["message"]["id"].as_i64().unwrap();

        println!("{}", remove_message);

        self.telegram
            .delete_message(self.cloud_chat_id, &vec![remove_message_id]).await
            .unwrap();

        Ok(())
    }

    async fn check_file(&self, file_name: &str) -> bool {
        todo!()
    }

    async fn close(self) -> Result<(), CloudError> {
        todo!()
    }
}
*/