// Copyright (c) 2022 Open Community Project Association https://ocpa.ch
// This software is published under the AGPLv3 license.

//! # Qaul File Sharing Service
//!
//! The File sharing service sends and receives file messages into the network.
//! The File messages carry on the Messaging service
//! Messaging(FileMessage(FileSharingContainer(FileInfo, FileData, Confirmation)))

//use bs58::decode;
use libp2p::PeerId;
use prost::Message;
use serde::{Deserialize, Serialize};
use sled_extensions::{bincode::Tree, DbExt};
use state::Storage;
use std::collections::BTreeMap;
use std::{
    convert::TryInto,
    io::{Read, Write},
    sync::RwLock,
};

use crate::node::user_accounts::{UserAccount, UserAccounts};

use crate::rpc::Rpc;
use crate::storage::database::DataBase;
use crate::utilities::timestamp;
use super::chat::Chat;

use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::Path;

use super::messaging::proto;
use super::messaging::Messaging;

/// Import protobuf message definition generated by
/// the rust module prost-build.
pub mod proto_rpc {
    include!("qaul.rpc.filesharing.rs");
}
pub mod proto_net {
    include!("qaul.net.filesharing.rs");
}

/// Structure to management for file histories based on the each user_id.
pub struct AllFiles {
    pub db_ref: BTreeMap<Vec<u8>, UserFiles>,
}

/// User file histories structure
#[derive(Clone)]
pub struct UserFiles {
    /// in memory BTreeMap
    pub histories: Tree<FileHistory>,

    /// last file index
    pub last_file: u64,
}

/// File history structure, this structure is stored into DB
#[derive(Serialize, Deserialize, Clone)]
pub struct FileHistory {
    /// peer user id 
    pub peer_id: Vec<u8>,
    /// file id
    pub id: u64,
    /// file name
    pub name: String,
    /// file description
    pub descr: String,
    /// file extension
    pub extension: String,
    /// file size in bytes
    pub size: u32,
    /// file sent or received time
    pub time: u64,
    /// false=> received, true=> sent
    pub sent: bool, 
}

/// mutable state of all file
static ALLFILES: Storage<RwLock<AllFiles>> = Storage::new();


/// mutable state for sending files
static FILESHARE: Storage<RwLock<FileShare>> = Storage::new();

/// mutable state for receiving files
static FILERECEIVE: Storage<RwLock<FileShareReceive>> = Storage::new();

/// pub const DEF_PACKAGE_SIZE: u32 = 64000; 
pub const DEF_PACKAGE_SIZE: u32 = 1000;

/// File Sharing information to management file transfering 
#[derive(Serialize, Deserialize, Clone)]
pub struct FileShareInfo {
    /// sender user id
    pub sender_id: Vec<u8>,
    /// recevier user id
    pub receiver_id: Vec<u8>,
    /// file size
    pub size: u32,
    /// file information message sent status
    pub sent_info: bool,
    /// file package messages status
    pub pkg_sent: Vec<u8>,
    /// last file data package size
    pub last_pkg_size: u32,
    /// file name
    pub name: String,
    /// file description
    pub descr: String,
    /// file extension
    pub extension: String,
    /// file identifier
    pub id: u64, 
    /// file trasnfering/receiving start time
    pub start_time: u64,
}

impl FileShareInfo {
    pub fn package_count_calc(size: u32, def_size: u32) -> u32 {
        let mut count = size / def_size;
        if size % def_size > 0 {
            count = count + 1;
        }
        return count;
    }

    /// This function check if trasfering or receving is finished.
    pub fn is_completed(&self) -> bool {
        if self.sent_info == false {
            return false;
        }

        for b in &self.pkg_sent {
            if *b == 0 {
                return false;
            }
        }
        true
    }    
}


/// For storing in data base
#[derive(Serialize, Deserialize, Clone)]
pub struct FileShare {
    pub files: BTreeMap<u64, FileShareInfo>,
}

/// For storing on the local storage receiving state
#[derive(Serialize, Deserialize, Clone)]
pub struct FileShareInfoReceving {
    pub info: FileShareInfo,

    /// package_seq => data
    pub packages: BTreeMap<u32, Vec<u8>>,
}

/// For storing in data base
#[derive(Serialize, Deserialize, Clone)]
pub struct FileShareReceive {
    pub files: BTreeMap<u64, FileShareInfoReceving>,
}

/// File sharing module to process transfer, receive and RPC commands
impl FileShare {
    /// initialize fileshare module
    pub fn init() {
        // create file transfer state
        let file_share = FileShare {
            files: BTreeMap::new(),
        };
        FILESHARE.set(RwLock::new(file_share));

        // create file receiver state
        let file_receive = FileShareReceive {
            files: BTreeMap::new(),
        };
        FILERECEIVE.set(RwLock::new(file_receive));

        // create file history state
        let all_files = AllFiles {
            db_ref: BTreeMap::new(),
        };
        ALLFILES.set(RwLock::new(all_files));
    }

    /// File history stored based on the user id.
    /// This function getting histrory table based on the user id.
    fn get_db_ref(user_id: &PeerId) -> UserFiles {
        // check if user data exists
        {
            // get chat state
            let all_files = ALLFILES.get().read().unwrap();

            // check if user ID is in map
            if let Some(user_files) = all_files.db_ref.get(&user_id.to_bytes()) {
                return UserFiles {
                    histories: user_files.histories.clone(),
                    last_file: user_files.last_file,
                };
            }
        }

        // create user data if it does not exist
        let user_files = Self::create_userfiles(user_id);

        // return chat_user structure
        UserFiles {
            histories: user_files.histories.clone(),
            last_file: user_files.last_file,
        }
    }

    /// create [user => file history] when it does not exist
    fn create_userfiles(user_id: &PeerId) -> UserFiles {
        // get user data base
        let db = DataBase::get_user_db(user_id.clone());

        // open trees
        let histories: Tree<FileHistory> = db.open_bincode_tree("file_sharing").unwrap();

        // get last key
        let last_file: u64;
        match histories.iter().last() {
            Some(Ok((ivec, _))) => {
                let i = ivec.to_vec();
                match i.try_into() {
                    Ok(arr) => {
                        last_file = u64::from_be_bytes(arr);
                    }
                    Err(e) => {
                        log::error!("couldn't convert ivec to u64: {:?}", e);
                        last_file = 0;
                    }
                }
            }
            None => {
                last_file = 0;
            }
            Some(Err(e)) => {
                log::error!("Sled feed table error: {}", e);
                last_file = 0;
            }
        }

        let user_files = UserFiles {
            histories,
            last_file,
        };

        // get chat state for writing
        let mut all_files = ALLFILES.get().write().unwrap();

        // add user to state
        all_files
            .db_ref
            .insert(user_id.to_bytes(), user_files.clone());

        // return structure
        user_files
    }

    /// This function is called when file transfer or receiving finished successfully.    
    fn on_completed(user_id: &PeerId, peer_id: &PeerId, info: &FileShareInfo, sent: bool) {
        let db_ref = Self::get_db_ref(user_id);

        let history = FileHistory {
            peer_id: peer_id.to_bytes(),
            id: info.id,
            name: info.name.clone(),
            descr: info.descr.clone(),
            extension: info.extension.clone(),
            size: info.size,
            time: timestamp::Timestamp::get_timestamp(),
            sent,
        };

        let last_file = db_ref.last_file + 1;

        // save to data base
        if let Err(e) = db_ref.histories.insert(&last_file.to_be_bytes(), history) {
            log::error!("Error saving feed message to data base: {}", e);
        } else {
            if let Err(e) = db_ref.histories.flush() {
                log::error!("Error when flushing data base to disk: {}", e);
            }
        }

        //update last_file
        let mut all_files = ALLFILES.get().write().unwrap();

        // check if user ID is in map
        if let Some(user_files) = all_files.db_ref.get_mut(&user_id.to_bytes()) {
            user_files.last_file = last_file;
        }


        //save chat messge
        if sent{
            Chat::save_outgoing_file_message(user_id.clone(), peer_id.to_bytes(), info.name.clone(), 
                info.size, last_file, info.id, info.descr.clone());
        }else{
            Chat::save_incoming_file_message(user_id.clone(), peer_id.clone(), info.name.clone(), 
                info.size, last_file, info.id, info.descr.clone());
        }

    }

    /// getting file extension from given filename
    fn get_extension_from_filename(filename: &str) -> Option<&str> {
        Path::new(filename).extension().and_then(OsStr::to_str)
    }

    /// Getting file histories from table.
    /// This function is called from RPC command (file history [offset limit])
    pub fn file_history(
        user_account: &UserAccount,
        history_req: &proto_rpc::FileHistoryRequest,
    ) -> (u64, Vec<FileHistory>) {
        let db_ref = Self::get_db_ref(&user_account.id);

        let mut histories: Vec<FileHistory> = vec![];

        if history_req.offset as u64 >= db_ref.last_file {
            //no histories from offset
            return (db_ref.last_file, histories);
        }

        let mut count = history_req.limit;
        if (history_req.offset + count) as u64 >= db_ref.last_file {
            count = (db_ref.last_file - (history_req.offset as u64)) as u32;
        }

        if count == 0 {
            //no histories from offset
            return (db_ref.last_file, histories);
        }

        let first_file = db_ref.last_file - ((history_req.offset + count) as u64) + 1;
        let end_file = first_file + (count as u64);
        let first_file_bytes = first_file.to_be_bytes().to_vec();
        let end_file_bytes = end_file.to_be_bytes().to_vec();

        for res in db_ref
            .histories
            .range(first_file_bytes.as_slice()..end_file_bytes.as_slice())
        {
            //for res in db_ref.histories.range(first_file_bytes.as_slice()..) {
            match res {
                Ok((_id, message)) => {
                    histories.push(message.clone());
                }
                Err(e) => {
                    log::error!("Error retrieving file history from data base: {}", e);
                }
            }
        }

        (db_ref.last_file, histories)
    }

    /// Send the file on the messaging service
    /// This function is called from RPC coomand (file send convrsation_id file_path_name)
    pub fn send(
        user_account: &UserAccount,
        sned_file_req: proto_rpc::SendFileRequest,
    ) -> Result<Vec<u8>, String> {
        // create receiver
        let receiver;
        match PeerId::from_bytes(&sned_file_req.conversation_id) {
            Ok(id) => receiver = id,
            Err(e) => return Err(e.to_string()),
        }

        if let Some(_usr) = UserAccounts::get_by_id(receiver.clone()) {
            //peer id is local user case
            log::error!("You cannot send the file to yourself.");
            return Ok(sned_file_req.conversation_id);
        }

        let mut file: File;

        match File::open(sned_file_req.path_name.clone()){
            Ok(f) => {Some(file = f)},
            Err(_e) => {
                return Err("file open error".to_string());
            }
        };

        let size = file.metadata().unwrap().len() as u32;
        if size == 0 {
            return Err("file size is zero".to_string());
        }

        //get file name
        let path = Path::new(sned_file_req.path_name.as_str());
        let mut extension = "".to_string();

        if let Some(ext) =
            Self::get_extension_from_filename(path.file_name().unwrap().to_str().unwrap())
        {
            extension = ext.to_string();
        }

        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();

        //get file id (senderid, receiver_id, filename, size)
        let key_bytes = Self::get_key_from_vec(
            user_account.id.to_bytes(),
            sned_file_req.conversation_id.clone(),
            sned_file_req.path_name.clone(),
            size,
            timestamp::Timestamp::get_timestamp(),
        );
        let file_id = crc::crc64::checksum_iso(&key_bytes);

        //copy file in files folder
        let path = std::env::current_dir().unwrap(); 
        let mut path_fies = path.as_path().to_str().unwrap().to_string();
        if path_fies.chars().last().unwrap() != '/'{
            path_fies.push_str("/");
        }
        path_fies.push_str(user_account.id.to_base58().as_str());
        path_fies.push_str("/files/");

        if let Err(e) = fs::create_dir_all(path_fies.clone()) {
            log::error!("creating folder error {}", e.to_string());
        }
        //copy file
        path_fies.push_str(file_id.to_string().as_str());
        if extension.len() > 0{
            path_fies.push_str(".");
            path_fies.push_str(&extension.clone().as_str());
        }
        if let Err(e) = fs::copy(sned_file_req.path_name.clone(), path_fies){
            log::error!("copy file error {}", e.to_string());            
        }

        //create file descriptor and save in storage
        let mut file_info = Self::create_file_share_info(
            &user_account.id.to_bytes(),
            &sned_file_req.conversation_id.clone(),
            file_id,
            size,
            DEF_PACKAGE_SIZE,
        );
        file_info.name = file_name.clone();
        file_info.descr = sned_file_req.description.clone();
        file_info.extension = extension.clone();

        let mut file_share = FILESHARE.get().write().unwrap();
        file_share.files.insert(file_info.id, file_info);

        // create fileInfo message and pack / send
        let file_share_message = proto_net::FileSharingContainer {
            message: Some(proto_net::file_sharing_container::Message::FileInfo(
                proto_net::FileSharingInfo {
                    file_name: file_name.clone(),
                    file_extension: extension.clone(),
                    file_size: size,
                    file_descr: sned_file_req.description.clone(),
                    size_per_package: DEF_PACKAGE_SIZE,
                    file_id,
                },
            )),
        };

        let mut message_buf = Vec::with_capacity(file_share_message.encoded_len());
        file_share_message
            .encode(&mut message_buf)
            .expect("Vec<u8> provides capacity as needed");
        Self::send_file_message_through_message(user_account, receiver, &message_buf);

        log::info!("sent file info message!");

        //read file contents and make FileData messages
        let mut buffer: [u8; DEF_PACKAGE_SIZE as usize] = [0; DEF_PACKAGE_SIZE as usize];
        let mut left_size = size;
        let mut seq: u32 = 0;
        while left_size > 0 {
            let mut read_size = left_size;
            if left_size > DEF_PACKAGE_SIZE {
                read_size = DEF_PACKAGE_SIZE;
            };
            //file.by_ref().take(read_size as u64).read(&mut buffer);
            if let Err(e) = file.read(&mut buffer) {
                return Err( e.to_string());
            }

            let file_data_message = proto_net::FileSharingContainer {
                message: Some(proto_net::file_sharing_container::Message::FileData(
                    proto_net::FileSharingData {
                        file_id,
                        sequence: seq,
                        file_size: size,
                        size_per_package: DEF_PACKAGE_SIZE,
                        data: buffer[0..(read_size as usize)].iter().cloned().collect(),
                    },
                )),
            };
            let mut message_buf0 = Vec::with_capacity(file_data_message.encoded_len());
            file_data_message
                .encode(&mut message_buf0)
                .expect("Vec<u8> provides capacity as needed");
            Self::send_file_message_through_message(user_account, receiver, &message_buf0);

            log::info!("sent file pkg message seq={}", seq);
            //increase seq
            seq = seq + 1;
            left_size = left_size - read_size;
        }
        Ok(sned_file_req.conversation_id)
    }


    /// Creating file info to management state on the storage
    fn create_file_share_info(sender_id: &Vec<u8>, receiver_id: &Vec<u8>, id: u64, size: u32, def_pkg_size: u32) -> FileShareInfo{
        let package_count = FileShareInfo::package_count_calc(size, def_pkg_size);
        let file_info = FileShareInfo {
            sender_id: sender_id.clone(),
            receiver_id: receiver_id.clone(),
            size,
            sent_info: false,
            pkg_sent: vec![0; package_count as usize],
            last_pkg_size: (size % def_pkg_size),
            name: "".to_string(),
            descr: "".to_string(),
            extension: "".to_string(),
            id,
            start_time: timestamp::Timestamp::get_timestamp(),
        };
        file_info
    }

    /// Create File key for generting file id 
    fn get_key_from_vec(sender_id: Vec<u8>, conversation_id: Vec<u8>, file_name: String, size: u32, timestamp: u64) -> Vec<u8> {        
        let mut name_bytes = file_name.as_bytes().to_vec();
        let mut size_bytes = size.to_be_bytes().to_vec();
        let mut time_bytes = timestamp.to_be_bytes().to_vec();
        let mut key_bytes = sender_id;
        let mut conversation_bytes = conversation_id;

        key_bytes.append(&mut conversation_bytes);
        key_bytes.append(&mut name_bytes);
        key_bytes.append(&mut size_bytes);
        key_bytes.append(&mut time_bytes);
        key_bytes
    }

    /// Send capsuled file message through messaging service
    fn send_file_message_through_message(user_account: &UserAccount, receiver:PeerId, data: &Vec<u8>){
        let snd_message = proto::Messaging{
            message: Some(proto::messaging::Message::FileMessage(
                proto::FileMessage{
                    content: data.to_vec(),
                }
            )),
        };
        let mut message_buf00 = Vec::with_capacity(snd_message.encoded_len());
        snd_message
            .encode(&mut message_buf00)
            .expect("Vec<u8> provides capacity as needed");
        log::info!("message_buf len {}", message_buf00.len());

        // send message via messaging
        if let Err(e) = Messaging::pack_and_send_message(user_account, receiver, message_buf00) {
            log::error!("file sending message failed {}", e.to_string());
        }
    }

    /// Check all file data received successfully, and store file if completed
    /// This function is called whenever receive file messae
    fn check_complete_and_store(file_receive: &FileShareInfoReceving) -> bool{
        //check if file receive completed
        if !file_receive.info.is_completed() {
            return false;
        }

        // check directory
        let path = std::env::current_dir().unwrap(); 
        let mut path_fies = path.as_path().to_str().unwrap().to_string();
        if path_fies.chars().last().unwrap() != '/'{
            path_fies.push_str("/");
        }
        path_fies.push_str(bs58::encode(file_receive.info.receiver_id.clone()).into_string().as_str());
        path_fies.push_str("/files/");

        if let Err(e) = fs::create_dir_all(path_fies.clone()) {
            log::error!("creating folder error {}", e.to_string());
        }
        
        // write all contents into real file        
        // let mut path = "./files/".to_string();
        path_fies.push_str(file_receive.info.id.to_string().as_str());
        if file_receive.info.extension.len() > 0{
            path_fies.push_str(".");
            path_fies.push_str(&file_receive.info.extension.as_str());
        }

        log::info!("storing file {}", path_fies.clone());
        let mut file: File = File::create(path_fies.clone()).unwrap();

        for i in 0..file_receive.info.pkg_sent.len() {
            let key: u32 = i as u32;
            if let Some(data) = file_receive.packages.get(&key) {
                if let Err(e) = file.write(&data){
                    log::error!("file storing failed {}", e.to_string());
                }
            }    
        }
        if let Err(e) = file.flush(){
            log::error!("file storing failed {}", e.to_string());
            return false;
        }
        true
    }

    /// OnReceive File info
    fn on_receive_file_info(user_account: &UserAccount, sender_id: PeerId, receiver_id: PeerId, file_info: &proto_net::FileSharingInfo){
        let mut file_receiver = FILERECEIVE.get().write().unwrap();

        // when file info message is arrived late than data messages.
        if file_receiver.files.contains_key(&file_info.file_id) == true {
            let mut file_receive = file_receiver.files.get_mut(&file_info.file_id).unwrap();
            if file_receive.info.sent_info == true {
                log::info!("file info already exists! id={}", file_info.file_id);
                return;
            }
            file_receive.info.sent_info = true;
            file_receive.info.extension = file_info.file_extension.clone();
            file_receive.info.descr = file_info.file_descr.clone();

            if Self::check_complete_and_store(&file_receive) {
                // store into database
                Self::on_completed(&receiver_id, &sender_id, &file_receive.info, false);

                // remove entry
                file_receiver.files.remove(&file_info.file_id);

                // send complete message                
                let completed = proto_net::FileSharingContainer {
                    message: Some(proto_net::file_sharing_container::Message::Completed(
                        proto_net::FileSharingCompleted {
                            file_id: file_info.file_id,
                        },
                    )),
                };
                let mut message_buff = Vec::with_capacity(completed.encoded_len());
                completed
                    .encode(&mut message_buff)
                    .expect("Vec<u8> provides capacity as needed");
                Self::send_file_message_through_message(user_account, sender_id, &message_buff);
                return;
            }
        } else {
            let mut file_inf = Self::create_file_share_info(
                &sender_id.to_bytes(),
                &receiver_id.to_bytes(),
                file_info.file_id,
                file_info.file_size,
                file_info.size_per_package,
            );
            file_inf.name = file_info.file_name.clone();
            file_inf.extension = file_info.file_extension.clone();
            file_inf.descr = file_info.file_descr.clone();
            file_inf.sent_info = true;

            let file_receive_info = FileShareInfoReceving {
                info: file_inf,
                packages: BTreeMap::new(),
            };
            file_receiver
                .files
                .insert(file_info.file_id, file_receive_info);
        }
        
        // send fileinfo confirm message
        let confim = proto_net::FileSharingContainer {
            message: Some(proto_net::file_sharing_container::Message::ConfirmationInfo(
                proto_net::FileSharingConfirmationFileInfo {
                    file_id: file_info.file_id
                }
            ))
        };    
        let mut message_buf = Vec::with_capacity(confim.encoded_len());
        confim
            .encode(&mut message_buf)
            .expect("Vec<u8> provides capacity as needed");
        Self::send_file_message_through_message(user_account, sender_id, &message_buf);
    }

    /// OnReceive File data
    fn on_receive_file_data(user_account: &UserAccount, sender_id: PeerId, receiver_id: PeerId, file_data: &proto_net::FileSharingData){
        let mut file_receiver = FILERECEIVE.get().write().unwrap();

        if file_receiver.files.contains_key(&file_data.file_id) == false {
            log::info!(
                "file info doesn't exist! creating info={}",
                file_data.file_id
            );

            let file_info = Self::create_file_share_info(
                &sender_id.to_bytes(),
                &receiver_id.to_bytes(),
                file_data.file_id,
                file_data.file_size,
                file_data.size_per_package,
            );
            let file_receive_info = FileShareInfoReceving {
                info: file_info,
                packages: BTreeMap::new(),
            };
            file_receiver
                .files
                .insert(file_data.file_id, file_receive_info);
        }

        //set flag
        let file_receive = file_receiver.files.get_mut(&file_data.file_id).unwrap();
        *file_receive.info.pkg_sent.get_mut(file_data.sequence as usize).unwrap() = 1;

        //keep data
        file_receive
            .packages
            .insert(file_data.sequence, file_data.data.clone());

        //send confirmation message.
        let confim = proto_net::FileSharingContainer {
            message: Some(proto_net::file_sharing_container::Message::Confirmation(
                proto_net::FileSharingConfirmation {
                    file_id: file_data.file_id,
                    sequence: file_data.sequence,
                },
            )),
        };
        let mut message_buf = Vec::with_capacity(confim.encoded_len());
        confim
            .encode(&mut message_buf)
            .expect("Vec<u8> provides capacity as needed");
        Self::send_file_message_through_message(user_account, sender_id, &message_buf);

        if file_data.sequence == 13 {
            log::error!("state sent_info {}", file_receive.info.sent_info);

            for f in &file_receive.info.pkg_sent {
                log::error!("state rcv pkg= {}", *f);
            }
        }

        //check if file receive completed
        if Self::check_complete_and_store(&file_receive) {
            //store into database
            Self::on_completed(&receiver_id, &sender_id, &file_receive.info, false);

            //remove entry
            file_receiver.files.remove(&file_data.file_id);

            //send complete
            let completed = proto_net::FileSharingContainer {
                message: Some(proto_net::file_sharing_container::Message::Completed(
                    proto_net::FileSharingCompleted {
                        file_id: file_data.file_id,
                    },
                )),
            };
            let mut message_buff = Vec::with_capacity(completed.encoded_len());
            completed
                .encode(&mut message_buff)
                .expect("Vec<u8> provides capacity as needed");
            Self::send_file_message_through_message(user_account, sender_id, &message_buff);
        }
    }

    /// OnReceive confirmation file info message
    fn on_receive_confirmation_file_info(_user_account: &UserAccount, sender_id: PeerId, receiver_id: PeerId, confirm: &proto_net::FileSharingConfirmationFileInfo){
        let mut file_sender = FILESHARE.get().write().unwrap();
        if file_sender.files.contains_key(&confirm.file_id) == false {
            log::info!("file info does not exist! id={}", confirm.file_id);
            return;
        }
        let mut file_info = file_sender.files.get_mut(&confirm.file_id).unwrap();
        file_info.sent_info = true;    
        
        //if file sending completed , we remove the entry
        if file_info.is_completed() {
            log::info!("file sent successfully id={}, size={}", file_info.id, file_info.size);
            Self::on_completed(&receiver_id, &sender_id, file_info, true);            
            file_sender.files.remove(&confirm.file_id);
        }
    }

    /// OnReceive confirmation file data message
    fn on_receive_confirmation_file_data(_user_account: &UserAccount, sender_id: PeerId, receiver_id: PeerId, confirm: &proto_net::FileSharingConfirmation){
        let mut file_sender = FILESHARE.get().write().unwrap();
        if file_sender.files.contains_key(&confirm.file_id) == false{
            log::info!("file info does not exist! id={}", confirm.file_id);
            return;
        }

        let file_info = file_sender.files.get_mut(&confirm.file_id).unwrap();
        if confirm.sequence < file_info.pkg_sent.len() as u32{
            *file_info.pkg_sent.get_mut(confirm.sequence as usize).unwrap() = 1;
        }

        //if file sending completed , we remove the entry
        if file_info.is_completed() {
            log::info!(
                "file sent successfully id={}, size={}",
                file_info.id,
                file_info.size
            );
            Self::on_completed(&receiver_id, &sender_id, file_info, true);
            file_sender.files.remove(&confirm.file_id);
        }
    } 
    
    /// OnReceive completed message of file transfer
    fn on_receive_completed(_user_account: &UserAccount, sender_id: PeerId, receiver_id: PeerId, completed: &proto_net::FileSharingCompleted){
        let mut file_sender = FILESHARE.get().write().unwrap();
        if file_sender.files.contains_key(&completed.file_id){
            if let Some(info) = file_sender.files.get(&completed.file_id){
                Self::on_completed(&receiver_id, &sender_id, info, true);
            }            
            file_sender.files.remove(&completed.file_id);            
        }

        let mut file_receiver = FILERECEIVE.get().write().unwrap();
        if file_receiver.files.contains_key(&completed.file_id){
            if let Some(info) = file_receiver.files.get(&completed.file_id){
                Self::on_completed(&receiver_id, &sender_id, &info.info, false);
            }            
            file_receiver.files.remove(&completed.file_id);
        }
    }     

    /// OnReceive cancel message of file transfer
    fn on_receive_canceled(_user_account: &UserAccount, _sender_id: PeerId, _receiver_id: PeerId, canceled: &proto_net::FileSharingCanceled){
        let mut file_sender = FILESHARE.get().write().unwrap();
        if file_sender.files.contains_key(&canceled.file_id) {
            file_sender.files.remove(&canceled.file_id);
        }

        let mut file_receiver = FILERECEIVE.get().write().unwrap();
        if file_receiver.files.contains_key(&canceled.file_id) {
            file_receiver.files.remove(&canceled.file_id);
        }
    }      
    
    /// OnReceive file messae procedure.
    pub fn on_receive_message(sender_id: PeerId, receiver_id: PeerId, data: Vec<u8>){
        //check receiver id is in users list
        let user;
        match UserAccounts::get_by_id(receiver_id) {
            Some(usr) => {
                user = usr;
            }
            None => {
                log::error!("no user id={}", receiver_id);
                return;
            }
        }

        match proto_net::FileSharingContainer::decode(&data[..]) {
            Ok(messaging) =>{
                match messaging.message{
                    Some(proto_net::file_sharing_container::Message::FileInfo(file_info)) => {
                        log::info!("file::on_receive_file_info");
                        Self::on_receive_file_info(&user, sender_id, receiver_id, &file_info);
                    },
                    Some(proto_net::file_sharing_container::Message::FileData(file_data)) => {
                        log::info!("file::on_receive_file_data seq={}", file_data.sequence);
                        Self::on_receive_file_data(&user, sender_id, receiver_id, &file_data);
                    },
                    Some(proto_net::file_sharing_container::Message::Confirmation(confirmation)) => {
                        log::info!("file::on_receive_confirmation_file_data seq={}", confirmation.sequence);
                        Self::on_receive_confirmation_file_data(&user, sender_id, receiver_id, &confirmation);
                    },
                    Some(proto_net::file_sharing_container::Message::ConfirmationInfo(confirmation)) => {
                        log::info!("file::on_receive_confirmation_file_info");
                        Self::on_receive_confirmation_file_info(&user, sender_id, receiver_id, &confirmation);
                    },
                    Some(proto_net::file_sharing_container::Message::Completed(completed)) => {
                        log::info!("file::on_completed");
                        Self::on_receive_completed(&user, sender_id, receiver_id, &completed);
                    },
                    Some(proto_net::file_sharing_container::Message::Canceled(canceled)) => {
                        log::info!("file::on_canceled");
                        Self::on_receive_canceled(&user, sender_id, receiver_id, &canceled);
                    },
                    None => {
                        log::error!("file share message from {} was empty", sender_id.to_base58())
                    }
                }
            },
            Err(e) => {
                log::error!(
                    "Error decoding FileSharing Message from {} to {}: {}",
                    sender_id.to_base58(),
                    receiver_id.to_base58(),
                    e
                );
            }
        }
    }

    /// Process incoming RPC request messages for file sharing module
    pub fn rpc(data: Vec<u8>, user_id: Vec<u8>) {
        let my_user_id = PeerId::from_bytes(&user_id).unwrap();

        match proto_rpc::FileSharing::decode(&data[..]) {
            Ok(filesharing) => {
                match filesharing.message {
                    Some(proto_rpc::file_sharing::Message::SendFileRequest(send_req)) => {
                        let user_account = UserAccounts::get_by_id(my_user_id).unwrap();

                        if let Err(e) = Self::send(&user_account, send_req){
                            log::error!("file rpc send file failed {}", e.to_string());
                        }                        
                    },
                    Some(proto_rpc::file_sharing::Message::FileHistory(history_req)) => {
                        let user_account = UserAccounts::get_by_id(my_user_id).unwrap();
                        log::error!("lib->file->history");
                        let (total, list) = Self::file_history(&user_account, &history_req);

                        let mut histories: Vec<proto_rpc::FileHistoryEntry> = vec![];
                        for entry in list {
                            let file_entry = proto_rpc::FileHistoryEntry {
                                file_id: entry.id,
                                file_name: entry.name.clone(),
                                file_ext: entry.extension.clone(),
                                file_size: entry.size,
                                file_descr: entry.descr.clone(),
                                time: entry.time,
                                sent: entry.sent,
                                peer_id: bs58::encode(entry.peer_id).into_string(),
                            };
                            histories.push(file_entry);
                        }

                        // pack message
                        let proto_message = proto_rpc::FileSharing {
                            message: Some(proto_rpc::file_sharing::Message::FileHistoryResponse(
                                proto_rpc::FileHistoryResponse {
                                    offset: history_req.offset,
                                    limit: history_req.limit,
                                    total,
                                    histories,
                                },
                            )),
                        };

                        // encode message
                        let mut buf = Vec::with_capacity(proto_message.encoded_len());
                        proto_message
                            .encode(&mut buf)
                            .expect("Vec<u8> provides capacity as needed");

                        // send message
                        Rpc::send_message(
                            buf,
                            crate::rpc::proto::Modules::Fileshare.into(),
                            "".to_string(),
                            Vec::new(),
                        );
                    }
                    _ => {
                        log::error!("Unhandled Protobuf File Message");
                    }
                }
            }
            Err(error) => {
                log::error!("{:?}", error);
            }
        }
    }
}