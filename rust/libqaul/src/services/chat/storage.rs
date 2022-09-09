// Copyright (c) 2021 Open Community Project Association https://ocpa.ch
// This software is published under the AGPLv3 license.

//! # Chat Conversations Storage Functions
//!
//! Saves and retrieves the chat conversations
//! and their overview from the data base.

use libp2p::PeerId;
use sled_extensions::{bincode::Tree, DbExt};
use state::Storage;
use std::collections::BTreeMap;
use std::sync::RwLock;

use super::rpc_proto;
use crate::router;
use crate::services::group::{self, conversation_id::ConversationId, GroupStorage};
use crate::storage::database::DataBase;
use crate::utilities::timestamp::Timestamp;

/// mutable state of chat messages
static CHAT: Storage<RwLock<ChatStorage>> = Storage::new();

/// chat DB references per user account
#[derive(Clone)]
pub struct ChatAccountDb {
    // chat conversations sled data base tree
    pub overview: Tree<rpc_proto::ChatOverview>,
    // messages sled data base tree
    pub messages: Tree<rpc_proto::ChatMessage>,
    // message id => db key
    pub message_ids: Tree<Vec<u8>>,
}

/// qaul Chat Conversation Storage
pub struct ChatStorage {
    /// data base tree references accessible
    /// by user account
    db_ref: BTreeMap<Vec<u8>, ChatAccountDb>,
}

impl ChatStorage {
    /// initialize chat storage
    pub fn init() {
        // create chat storage
        let chat = ChatStorage {
            db_ref: BTreeMap::new(),
        };
        CHAT.set(RwLock::new(chat));
    }

    /// check if messages exists
    pub fn is_messages_exist(user_id: &PeerId, message_ids: &Vec<Vec<u8>>) -> bool {
        // get data base of user account
        let db_ref = Self::get_db_ref(user_id.clone());
        for id in message_ids {
            if !db_ref.message_ids.contains_key(id).unwrap() {
                return false;
            }
        }
        true
    }

    /// get messages by ids
    pub fn get_messages_by_id(user_id: &PeerId, message_ids: &Vec<Vec<u8>>) -> Vec<(i32, Vec<u8>)> {
        let mut res: Vec<(i32, Vec<u8>)> = vec![];

        let db_ref = Self::get_db_ref(user_id.clone());
        for id in message_ids {
            match db_ref.message_ids.get(id) {
                Ok(opt_key) => {
                    if let Some(db_key) = opt_key {
                        match db_ref.messages.get(&db_key) {
                            Ok(opt_msg) => {
                                if let Some(msg) = opt_msg {
                                    res.push((msg.content_type, msg.content.clone()));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        res
    }

    /// remove messages by ids
    pub fn remove_messages(user_id: &PeerId, message_ids: &Vec<Vec<u8>>) {
        let db_ref = Self::get_db_ref(user_id.clone());
        for id in message_ids {
            match db_ref.message_ids.get(id) {
                Ok(opt_key) => {
                    if let Some(db_key) = opt_key {
                        if let Err(_e) = db_ref.messages.remove(&db_key) {
                            log::error!("remove message error!");
                        }
                    }
                    if let Err(_e) = db_ref.message_ids.remove(id) {
                        log::error!("remove message id error!");
                    }
                }
                _ => {}
            }
        }

        if let Err(_e) = db_ref.messages.flush() {
            log::error!("message storing error!");
        }
        if let Err(_e) = db_ref.message_ids.flush() {
            log::error!("message ids storing error!");
        }
    }

    /// Save message
    ///
    /// This function saves a message into the data base
    pub fn save_event(
        account_id: &PeerId,
        sender_id: &PeerId,
        content_type: i32,
        content: &Vec<u8>,
        conversation_id: &ConversationId,
    ) -> bool {
        // create timestamp
        let timestamp = Timestamp::get_timestamp();

        // get data base of user account
        let db_ref = Self::get_db_ref(account_id.clone());

        // check if group exists
        if !GroupStorage::group_exists(account_id.to_owned(), conversation_id.to_bytes()) {
            let test_conversation_id = ConversationId::from_peers(account_id, sender_id);

            // check if it is a direct chat
            if conversation_id == &test_conversation_id {
                // create new group
                group::Manage::create_new_direct_chat_group(account_id, sender_id);
            } else {
                // group is unknown
                return false;
            }
        }

        // check if it is a direct group
        let is_group = !(conversation_id == &ConversationId::from_peers(account_id, sender_id));

        let overview;
        match Self::update_overview(
            account_id,
            sender_id,
            &db_ref,
            &conversation_id.to_bytes(),
            timestamp,
            content_type,
            content,
            &sender_id.to_bytes(),
            is_group,
        ) {
            Ok(chat_overview) => overview = chat_overview,
            Err(e) => {
                log::error!("{}", e);
                return false;
            }
        }

        // create data base key
        let db_key =
            Self::get_db_key_from_vec(&conversation_id.to_bytes(), overview.last_message_index);

        // create chat message
        let chat_message = rpc_proto::ChatMessage {
            index: overview.last_message_index,
            sender_id: sender_id.to_bytes(),
            message_id: vec![],
            status: 0,
            message_reception_confirmed: Vec::new(),
            conversation_id: conversation_id.to_bytes(),
            sent_at: timestamp,
            received_at: timestamp,
            content_type,
            content: content.clone(),
        };

        // save message in data base
        if let Err(e) = db_ref.messages.insert(db_key.clone(), chat_message) {
            log::error!("Error saving chat message to data base: {}", e);
        }
        // flush trees to disk
        if let Err(e) = db_ref.messages.flush() {
            log::error!("Error chat messages flush: {}", e);
        }

        true
    }

    /// Save a new incoming Message
    ///
    /// This function saves an incoming chat message in the data base
    pub fn save_incoming_message(
        account_id: &PeerId,
        sender_id: &PeerId,
        content_type: i32,
        content: Vec<u8>,
        sent_at: u64,
        conversation_id: &ConversationId,
        message_id: &Vec<u8>,
        status: rpc_proto::MessageStatus,
    ) -> bool {
        // create timestamp
        let timestamp = Timestamp::get_timestamp();

        // get data base of user account
        let db_ref = Self::get_db_ref(account_id.clone());

        // check if message_id is already exist
        if db_ref.message_ids.contains_key(message_id).unwrap() {
            return true;
        }

        // check if group message
        let is_group = !(conversation_id == &ConversationId::from_peers(account_id, sender_id));

        // if direct chat, check group exist
        if !is_group {
            if !GroupStorage::group_exists(account_id.to_owned(), conversation_id.to_bytes()) {
                group::Manage::create_new_direct_chat_group(account_id, sender_id);
            }
        }

        log::trace!("save incoming is_group={}", is_group);
        let overview;
        match Self::update_overview(
            account_id,
            sender_id,
            &db_ref,
            &conversation_id.to_bytes(),
            timestamp,
            content_type,
            &content,
            &sender_id.to_bytes(),
            is_group,
        ) {
            Ok(chat_overview) => overview = chat_overview,
            Err(e) => {
                log::error!("{}", e);
                return false;
            }
        }

        // create data base key
        let db_key =
            Self::get_db_key_from_vec(&conversation_id.to_bytes(), overview.last_message_index);

        // create chat message
        let chat_message = rpc_proto::ChatMessage {
            index: overview.last_message_index,
            sender_id: sender_id.to_bytes(),
            message_id: message_id.clone(),
            status: status.try_into().unwrap(),
            message_reception_confirmed: Vec::new(),
            conversation_id: conversation_id.to_bytes(),
            sent_at,
            received_at: timestamp,
            content_type: content_type,
            content,
        };

        // save message in data base
        if let Err(e) = db_ref.messages.insert(db_key.clone(), chat_message) {
            log::error!("Error saving chat message to data base: {}", e);
        }
        // flush trees to disk
        if let Err(e) = db_ref.messages.flush() {
            log::error!("Error chat messages flush: {}", e);
        }

        // save message id in data base
        if let Err(e) = db_ref
            .message_ids
            .insert(message_id.clone(), db_key.clone())
        {
            log::error!("Error saving chat messageid to data base: {}", e);
        }
        // flush trees to disk
        if let Err(e) = db_ref.message_ids.flush() {
            log::error!("Error chat message_ids flush: {}", e);
        }
        true
    }

    /// save an outgoing message to the data base
    pub fn save_outgoing_message(
        account_id: &PeerId,
        receiver_id: &PeerId,
        conversation_id: &ConversationId,
        message_id: &Vec<u8>,
        content_type: i32,
        content: &Vec<u8>,
        status: rpc_proto::MessageStatus,
    ) {
        // create timestamp
        let timestamp = Timestamp::get_timestamp();

        // get data base of user account
        let db_ref = Self::get_db_ref(account_id.clone());

        // check if message_id is already exist
        if db_ref.message_ids.contains_key(message_id).unwrap() {
            return;
        }

        // check if group message
        let is_group = !(conversation_id == &ConversationId::from_peers(account_id, receiver_id));

        // if direct chat, check group exist
        if !is_group {
            if !GroupStorage::group_exists(account_id.to_owned(), conversation_id.to_bytes()) {
                group::Manage::create_new_direct_chat_group(account_id, receiver_id);
            }
        }

        // get overview
        let overview;
        match Self::update_overview(
            account_id,
            receiver_id,
            &db_ref,
            &conversation_id.to_bytes(),
            timestamp,
            content_type,
            content,
            &account_id.to_bytes(),
            is_group,
        ) {
            Ok(chat_overview) => overview = chat_overview,
            Err(e) => {
                log::error!("{}", e);
                return;
            }
        }

        // create data base key
        let key =
            Self::get_db_key_from_vec(&conversation_id.to_bytes(), overview.last_message_index);

        // // create chat message
        let message = rpc_proto::ChatMessage {
            index: overview.last_message_index,
            sender_id: account_id.to_bytes(),
            message_id: message_id.clone(),
            status: status.try_into().unwrap(),
            message_reception_confirmed: Vec::new(),
            conversation_id: conversation_id.to_bytes(),
            sent_at: timestamp,
            received_at: timestamp,
            content_type,
            content: content.clone(),
        };

        // save message in data base
        if let Err(e) = db_ref.messages.insert(key.clone(), message) {
            log::error!("Error saving chat message to data base: {}", e);
        }

        // flush trees to disk
        if let Err(e) = db_ref.messages.flush() {
            log::error!("Error chat messages flush: {}", e);
        }

        if let Err(e) = db_ref.message_ids.insert(message_id.clone(), key.clone()) {
            log::error!("Error saving chat messageid to data base: {}", e);
        }

        // flush trees to disk
        if let Err(e) = db_ref.message_ids.flush() {
            log::error!("Error chat message_ids flush: {}", e);
        }
    }

    /// updating chat message status as confirmed
    pub fn update_confirmation(
        account_id: PeerId,
        receiver_id: PeerId,
        message_id: &Vec<u8>,
        received_at: u64,
    ) {
        // get data base of user account
        let db_ref = Self::get_db_ref(account_id.clone());
        if let Some(key) = db_ref.message_ids.get(message_id).unwrap() {
            if let Some(mut chat_msg) = db_ref.messages.get(&key).unwrap() {
                chat_msg.status = rpc_proto::MessageStatus::Confirmed as i32;
                chat_msg.received_at = received_at;

                // TODO: check if receiver already exists

                // receiving user
                let mut confirmation = rpc_proto::MessageReceptionConfirmed {
                    user_id: receiver_id.to_bytes(),
                    confirmed_at: received_at,
                };
                chat_msg.message_reception_confirmed.push(confirmation);

                // TODO: check if it was received by everyone
                //       set received_by_all flag if yes

                // save message in data base
                if let Err(e) = db_ref.messages.insert(key.clone(), chat_msg) {
                    log::error!("Error saving chat message to data base: {}", e);
                }
                // flush trees to disk
                if let Err(e) = db_ref.messages.flush() {
                    log::error!("Error chat messages flush: {}", e);
                }
            }
        }
    }

    /// Update the last Message and the Conversation Index of an Overview entry
    fn update_overview(
        account_id: &PeerId,
        peer_id: &PeerId,
        db_ref: &ChatAccountDb,
        conversation_id: &Vec<u8>,
        timestamp: u64,
        content_type: i32,
        content: &Vec<u8>,
        last_message_sender_id: &Vec<u8>,
        b_group: bool,
    ) -> Result<rpc_proto::ChatOverview, String> {
        // check if there is an conversation
        let mut overview: rpc_proto::ChatOverview;

        match db_ref.overview.get(conversation_id.clone()) {
            // conversation exists
            Ok(Some(my_conversation)) => {
                overview = my_conversation;

                // update conversation
                overview.last_message_index = overview.last_message_index + 1;
                overview.last_message_at = timestamp;
                overview.unread = overview.unread + 1;
                overview.content = content.clone();
                overview.content_type = content_type;
                overview.last_message_sender_id = last_message_sender_id.clone();
            }
            // conversation does not exist yet
            Ok(None) => {
                // get user name from known users
                let name;
                if b_group {
                    if let Some(group_name) =
                        super::group::Group::get_group_name(account_id, &conversation_id)
                    {
                        name = group_name.clone();
                    } else {
                        return Err("Group not found".to_string());
                    }
                } else {
                    match router::users::Users::get_name(peer_id) {
                        Some(username) => name = username,
                        None => {
                            return Err("User not found".to_string());
                        }
                    }
                }

                // create a new conversation
                overview = rpc_proto::ChatOverview {
                    conversation_id: conversation_id.clone(),
                    last_message_index: 1,
                    name,
                    last_message_at: timestamp,
                    unread: 1,
                    content_type,
                    content: content.clone(),
                    last_message_sender_id: last_message_sender_id.clone(),
                };
            }
            // data base error
            Err(e) => {
                log::error!("{}", e);
                return Err("Error fetching conversation from data base".to_string());
            }
        }

        // save conversation overview in data base
        if let Err(e) = db_ref
            .overview
            .insert(conversation_id.clone(), overview.clone())
        {
            log::error!("{}", e);
            return Err("Error saving chat overview to data base".to_string());
        }

        // flush tree to disk
        if let Err(e) = db_ref.overview.flush() {
            log::error!("Error chat overview flush: {}", e);
        }

        Ok(overview)
    }

    /// Get conversation overview list from data base
    pub fn get_overview(user_id: PeerId) -> rpc_proto::ChatOverviewList {
        // create empty conversation list
        let mut overview_list: Vec<rpc_proto::ChatOverview> = Vec::new();

        // get chat conversations overview tree for user
        let db_ref = Self::get_db_ref(user_id);

        // iterate over all values in db
        for res in db_ref.overview.iter() {
            if let Ok((_vec, conversation)) = res {
                overview_list.push(conversation);
            }
        }

        rpc_proto::ChatOverviewList { overview_list }
    }

    /// Get chat messages of a specific conversation from data base
    pub fn get_messages(
        account_id: PeerId,
        conversation_id: Vec<u8>,
    ) -> rpc_proto::ChatConversationList {
        // create empty messages list
        let mut message_list: Vec<rpc_proto::ChatMessage> = Vec::new();

        if conversation_id.len() == 16 {
            // get database references for this user account
            let db_ref = Self::get_db_ref(account_id);

            // create message keys
            let (first_key, last_key) = Self::get_db_key_range(&conversation_id.clone());

            // iterate over all values in chat_messages db
            for res in db_ref
                .messages
                .range(first_key.as_slice()..last_key.as_slice())
            {
                match res {
                    Ok((_id, message)) => {
                        // check message type and filter for all type none messages
                        //
                        // rpc_proto::ChatContentType::None = 0
                        match message.content_type {
                            0 => {
                                log::error!("Content type was None")
                            }
                            _ => message_list.push(message),
                        }
                    }
                    Err(e) => {
                        log::error!("get_messages error: {}", e);
                    }
                }
            }
        }

        rpc_proto::ChatConversationList {
            conversation_id,
            message_list,
        }
    }

    /// get DB key range for a conversation ID
    ///
    /// returns a key tuple, which can be used to
    /// retrieve all messages for a user ID from the DB:
    ///
    /// (first_key, last_key)
    fn get_db_key_range(conversation_id: &Vec<u8>) -> (Vec<u8>, Vec<u8>) {
        let first_key = Self::get_db_key_from_vec(conversation_id, 0);
        let last_key = Self::get_db_key_from_vec(conversation_id, 0xFFFFFFFFFFFFFFFF); // = 4294967295
        (first_key, last_key)
    }

    /// create DB key from conversation ID
    fn get_db_key_from_vec(conversation_id: &Vec<u8>, index: u64) -> Vec<u8> {
        let mut index_bytes = index.to_be_bytes().to_vec();
        let mut key_bytes = conversation_id.clone();
        key_bytes.append(&mut index_bytes);
        key_bytes
    }

    /// get user account data base tree references
    fn get_db_ref(account_id: PeerId) -> ChatAccountDb {
        // check if user account data exists
        {
            // get chat state
            let chat = CHAT.get().read().unwrap();

            // check if user account ID is in map
            if let Some(chat_user) = chat.db_ref.get(&account_id.to_bytes()) {
                return ChatAccountDb {
                    overview: chat_user.overview.clone(),
                    messages: chat_user.messages.clone(),
                    message_ids: chat_user.message_ids.clone(),
                };
            }
        }

        // create user data if it does not exist
        let chat_user = Self::create_chatuser(account_id);

        // return chat_user structure
        ChatAccountDb {
            overview: chat_user.overview.clone(),
            messages: chat_user.messages.clone(),
            message_ids: chat_user.message_ids.clone(),
        }
    }

    /// create user data when it does not exist
    fn create_chatuser(account_id: PeerId) -> ChatAccountDb {
        // get user data base
        let db = DataBase::get_user_db(account_id);

        // open trees
        let overview: Tree<rpc_proto::ChatOverview> =
            db.open_bincode_tree("chat_overview").unwrap();
        let messages: Tree<rpc_proto::ChatMessage> = db.open_bincode_tree("chat_messages").unwrap();
        let message_ids: Tree<Vec<u8>> = db.open_bincode_tree("chat_message_ids").unwrap();

        let chat_user = ChatAccountDb {
            overview,
            messages,
            message_ids,
        };

        // get chat state for writing
        let mut chat = CHAT.get().write().unwrap();

        // add user to state
        chat.db_ref.insert(account_id.to_bytes(), chat_user.clone());

        // return structure
        chat_user
    }
}