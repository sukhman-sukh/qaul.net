// Copyright (c) 2021 Open Community Project Association https://ocpa.ch
// This software is published under the AGPLv3 license.

//! # Chat Module
//!
//! Send and receive chat messages

use libp2p::PeerId;
use prost::Message;
use sled_extensions::{bincode::Tree, DbExt};
use state::Storage;
use std::collections::BTreeMap;
use std::sync::RwLock;

use super::messaging;
use super::messaging::proto;
use super::messaging::ConversationId;
use super::messaging::Messaging;
use crate::connections::{internet::Internet, lan::Lan};
use crate::node::user_accounts::{UserAccount, UserAccounts};
use crate::router;
use crate::rpc::Rpc;
use crate::storage::database::DataBase;
use crate::utilities::timestamp::Timestamp;

/// Import protobuf message definition generated by
/// the rust module prost-build.
pub mod rpc_proto {
    include!("qaul.rpc.chat.rs");
}
use super::group;

/// mutable state of chat messages
static CHAT: Storage<RwLock<Chat>> = Storage::new();

/// chat references per user account
#[derive(Clone)]
pub struct ChatUser {
    // chat conversations sled data base tree
    pub overview: Tree<rpc_proto::ChatOverview>,
    // messages sled data base tree
    pub messages: Tree<rpc_proto::ChatMessage>,
    // message id => db key
    pub message_ids: Tree<Vec<u8>>,
}

/// qaul Chat storage and logic
pub struct Chat {
    // data base tree references accessible
    // by user account
    db_ref: BTreeMap<Vec<u8>, ChatUser>,
}

impl Chat {
    /// initialize chat module
    pub fn init() {
        // create chat state
        let chat = Chat {
            db_ref: BTreeMap::new(),
        };
        CHAT.set(RwLock::new(chat));
    }

    /// Save a new incoming Message
    ///
    /// This function saves an incoming chat message in the data base
    pub fn save_incoming_message(
        user_id: &PeerId,
        sender_id: &PeerId,
        content: &Vec<u8>,
        sent_at: u64,
        conversation_id: &messaging::ConversationId,
        message_id: &Vec<u8>,
        status: u32,
    ) -> bool {
        // create timestamp
        let timestamp = Timestamp::get_timestamp();

        // get data base of user account
        let db_ref = Self::get_user_db_ref(user_id.clone());

        // check if message_id is already exist
        if db_ref.message_ids.contains_key(message_id).unwrap() {
            return true;
        }

        // check if group message
        let is_group = !(conversation_id
            .is_equal(&messaging::ConversationId::from_peers(user_id, sender_id).unwrap()));

        // if direct chat, check group exist
        if !is_group {
            if !group::Group::is_group_exist(user_id, &conversation_id.to_bytes()) {
                group::Manage::create_new_direct_chat_group(user_id, sender_id);
            }
        }

        log::error!("save incomeming is_group={}", is_group);
        let overview;
        match Self::update_overview(
            user_id,
            sender_id,
            &db_ref,
            &conversation_id.to_bytes(),
            timestamp,
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
            message_id: message_id.clone(),
            status,
            is_group,
            conversation_id: conversation_id.to_bytes(),
            sent_at,
            received_at: timestamp,
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

        //save message id in data base
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

    // send message
    pub fn send(
        user_account: &UserAccount,
        receiver: &PeerId,
        common_message: &proto::CommonMessage,
    ) -> Result<Vec<u8>, String> {
        let send_message = proto::Messaging {
            message: Some(proto::messaging::Message::CommonMessage(
                common_message.clone(),
            )),
        };
        Messaging::pack_and_send_message(
            user_account,
            &receiver,
            &send_message.encode_to_vec(),
            Some(&common_message.message_id),
            true,
        )
    }

    // send the message
    pub fn send_to_peer(
        user_account: &UserAccount,
        chat_message: rpc_proto::ChatMessageSend,
    ) -> Result<bool, String> {
        let mut conversation_id;
        //direct chat messge case.
        if chat_message.conversation_id.len() > 16 {
            // create new room if no exist
            let peer_id = PeerId::from_bytes(&chat_message.conversation_id).unwrap();
            if user_account.id == peer_id {
                return Err("You can not send message to yourself".to_string());
            }

            let group_id =
                messaging::ConversationId::from_peers(&user_account.id, &peer_id).unwrap();
            if !group::Group::is_group_exist(&user_account.id, &group_id.to_bytes()) {
                group::Manage::create_new_direct_chat_group(&user_account.id, &peer_id);
            }
            conversation_id = group_id.to_bytes();
        } else {
            conversation_id = chat_message.conversation_id.clone();
        }
        Self::send_message(
            &user_account.id,
            &conversation_id,
            chat_message.content.clone(),
        )
    }

    // send group message
    fn send_message(
        my_user_id: &PeerId,
        group_id: &Vec<u8>,
        message: String,
    ) -> Result<bool, String> {
        let group;
        match group::Group::get_group(my_user_id, group_id) {
            Ok(v) => {
                group = v;
            }
            Err(error) => {
                return Err(error);
            }
        }

        let mut my_member;
        match group.get_member(&my_user_id.to_bytes()) {
            Some(v) => {
                my_member = v.clone();
            }
            _ => {
                return Err("you are not member in this group".to_string());
            }
        }

        let last_index = my_member.last_message_index + 1;
        let timestamp = Timestamp::get_timestamp();
        let conversation_id = super::messaging::ConversationId::from_bytes(&group.id).unwrap();
        let message_id = super::messaging::Messaging::generate_group_message_id(
            &group.id, my_user_id, last_index,
        );

        // pack message
        let common_message = proto::CommonMessage {
            message_id: message_id.clone(),
            conversation_id: conversation_id.to_bytes(),
            sent_at: timestamp,
            payload: Some(proto::common_message::Payload::ChatMessage(
                proto::ChatMessage {
                    content: message.clone(),
                },
            )),
        };

        // save outgoing message
        Chat::save_outgoing_message(
            my_user_id,
            my_user_id,
            &conversation_id,
            &message_id,
            &common_message.encode_to_vec(),
            0,
        );

        //broad cast to all group members
        if let Some(user_account) = UserAccounts::get_by_id(my_user_id.clone()) {
            for user_id in group.members.keys() {
                let receiver = PeerId::from_bytes(&user_id.clone()).unwrap();
                if receiver != *my_user_id {
                    log::error!("send message to {}", receiver.to_base58());
                    if let Err(error) = Self::send(&user_account, &receiver, &common_message) {
                        log::error!("sending group message error {}", error);
                    }
                }
            }
        }

        //update member state
        my_member.last_message_index = last_index;
        group::Group::update_group_member(my_user_id, group_id, &my_member);
        Ok(true)
    }

    // save an outgoing message to the data base
    pub fn save_outgoing_message(
        user_id: &PeerId,
        receiver_id: &PeerId,
        conversation_id: &messaging::ConversationId,
        message_id: &Vec<u8>,
        content: &Vec<u8>,
        status: u32,
    ) {
        // // create timestamp
        let timestamp = Timestamp::get_timestamp();

        // // get data base of user account
        let db_ref = Self::get_user_db_ref(user_id.clone());

        // check if message_id is already exist
        if db_ref.message_ids.contains_key(message_id).unwrap() {
            return;
        }

        // check if group message
        let is_group = !(conversation_id
            .is_equal(&messaging::ConversationId::from_peers(user_id, receiver_id).unwrap()));

        // get overview
        let overview;
        match Self::update_overview(
            user_id,
            receiver_id,
            &db_ref,
            &conversation_id.to_bytes(),
            timestamp,
            content,
            &user_id.to_bytes(),
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
            sender_id: user_id.to_bytes(),
            message_id: message_id.clone(),
            status,
            is_group,
            conversation_id: conversation_id.to_bytes(),
            sent_at: timestamp,
            received_at: timestamp,
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

    // save an outgoing chat message to the data base
    fn save_outgoing_chat_message(
        user_id: PeerId,
        conversation_id: Vec<u8>,
        content: String,
        signature: Vec<u8>,
    ) {
        // let contents = rpc_proto::ChatMessageContent{
        //     content: Some(
        //         rpc_proto::chat_message_content::Content::ChatContent(
        //             rpc_proto::ChatContent{content}
        //         )
        //     )
        // };
        // Self::save_outgoing_message(user_id, conversation_id, contents.encode_to_vec(), signature, 0)
    }

    // save a sent file message to the data base
    pub fn save_outgoing_file_message(
        user_id: PeerId,
        conversation_id: Vec<u8>,
        file_name: String,
        file_size: u32,
        history_index: u64,
        file_id: u64,
        file_descr: String,
    ) {
        // let contents = rpc_proto::ChatMessageContent{
        //     content: Some(
        //         rpc_proto::chat_message_content::Content::FileContent(
        //             rpc_proto::FileShareContent{
        //                 history_index,
        //                 file_id,
        //                 file_name,
        //                 file_size,
        //                 file_descr,
        //             }
        //         )
        //     )
        // };
        // Self::save_outgoing_message(user_id, conversation_id, contents.encode_to_vec(), vec![], 1);
    }

    pub fn save_outgoing_group_invite_message(
        user_id: PeerId,
        conversation_id: PeerId,
        group_id: &Vec<u8>,
        group_name: String,
        created_at: u64,
        admin_id: &Vec<u8>,
        member_count: u32,
    ) {
        // let contents = rpc_proto::ChatMessageContent{
        //     content: Some(
        //         rpc_proto::chat_message_content::Content::GroupInviteContent(
        //             rpc_proto::GroupInviteContent{
        //                 group_id: group_id.clone(),
        //                 group_name: group_name.clone(),
        //                 created_at,
        //                 member_count,
        //                 admin_id: admin_id.clone()
        //             }
        //         )
        //     )
        // };
        // Self::save_outgoing_message(user_id, conversation_id.to_bytes(), contents.encode_to_vec(), vec![], 1);
    }
    pub fn save_outgoing_group_invite_reply_message(
        user_id: PeerId,
        conversation_id: PeerId,
        group_id: &Vec<u8>,
        accept: bool,
    ) {
        // let contents = rpc_proto::ChatMessageContent{
        //     content: Some(
        //         rpc_proto::chat_message_content::Content::GroupInviteReplyContent(
        //             rpc_proto::GroupInviteReplyContent{
        //                 group_id: group_id.clone(),
        //                 accept,
        //             }
        //         )
        //     )
        // };
        // Self::save_outgoing_message(user_id, conversation_id.to_bytes(), contents.encode_to_vec(), vec![], 1);
    }

    /// updating chat messge status as confirmed
    pub fn update_confirmation(user_id: &PeerId, message_id: &Vec<u8>, received_at: u64) {
        // get data base of user account
        let db_ref = Self::get_user_db_ref(user_id.clone());
        if let Some(key) = db_ref.message_ids.get(message_id).unwrap() {
            if let Some(mut chat_msg) = db_ref.messages.get(&key).unwrap() {
                chat_msg.status = 1;
                chat_msg.received_at = received_at;

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
        user_id: &PeerId,
        peer_id: &PeerId,
        db_ref: &ChatUser,
        conversation_id: &Vec<u8>,
        timestamp: u64,
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
                overview.last_message_sender_id = last_message_sender_id.clone();
            }
            // conversation does not exist yet            unconfirmed: chat_user.unconfirmed.clone(),
            Ok(None) => {
                // get user name from known users
                let name;
                if b_group {
                    if let Some(group_name) =
                        super::group::Group::get_group_name(user_id, &conversation_id)
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
    fn get_overview(user_id: PeerId) -> rpc_proto::ChatOverviewList {
        // create empty conversation list
        let mut overview_list: Vec<rpc_proto::ChatOverview> = Vec::new();

        // get chat conversations overview tree for user
        let db_ref = Self::get_user_db_ref(user_id);

        // iterate over all values in db
        for res in db_ref.overview.iter() {
            if let Ok((_vec, conversation)) = res {
                overview_list.push(conversation);
            }
        }

        rpc_proto::ChatOverviewList { overview_list }
    }

    /// Get chat messages of a specific conversation from data base
    fn get_messages(user_id: PeerId, conversation_id: Vec<u8>) -> rpc_proto::ChatConversationList {
        // create empty messages list
        let mut message_list: Vec<rpc_proto::ChatMessage> = Vec::new();

        // get database references for this user account
        let db_ref = Self::get_user_db_ref(user_id);

        let mut conversation_id0 = conversation_id.clone();
        log::error!("id len={}", conversation_id.len());
        if conversation_id.len() > 16 {
            conversation_id0 = messaging::ConversationId::from_peers(
                &user_id,
                &PeerId::from_bytes(&conversation_id.clone()).unwrap(),
            )
            .unwrap()
            .to_bytes();
        }

        log::error!(
            "conversation id ={}",
            bs58::encode(conversation_id0.clone()).into_string()
        );
        // create message keys
        let (first_key, last_key) = Self::get_db_key_range(&conversation_id0.clone());

        // iterate over all values in chat_messages db
        for res in db_ref
            .messages
            .range(first_key.as_slice()..last_key.as_slice())
        {
            match res {
                Ok((_id, message)) => {
                    message_list.push(message);
                }
                Err(e) => {
                    log::error!("get_messages error: {}", e);
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

    // create DB key from conversation ID
    fn get_db_key_from_vec(conversation_id: &Vec<u8>, index: u64) -> Vec<u8> {
        let mut index_bytes = index.to_be_bytes().to_vec();
        let mut key_bytes = conversation_id.clone();
        key_bytes.append(&mut index_bytes);
        key_bytes
    }

    // get user data base tree references
    fn get_user_db_ref(user_id: PeerId) -> ChatUser {
        // check if user data exists
        {
            // get chat state
            let chat = CHAT.get().read().unwrap();

            // check if user ID is in map
            if let Some(chat_user) = chat.db_ref.get(&user_id.to_bytes()) {
                return ChatUser {
                    overview: chat_user.overview.clone(),
                    messages: chat_user.messages.clone(),
                    message_ids: chat_user.message_ids.clone(),
                };
            }
        }

        // create user data if it does not exist
        let chat_user = Self::create_chatuser(user_id);

        // return chat_user structure
        ChatUser {
            overview: chat_user.overview.clone(),
            messages: chat_user.messages.clone(),
            message_ids: chat_user.message_ids.clone(),
        }
    }

    // create user data when it does not exist
    fn create_chatuser(user_id: PeerId) -> ChatUser {
        // get user data base
        let db = DataBase::get_user_db(user_id);

        // open trees
        let overview: Tree<rpc_proto::ChatOverview> =
            db.open_bincode_tree("chat_overview").unwrap();
        let messages: Tree<rpc_proto::ChatMessage> = db.open_bincode_tree("chat_messages").unwrap();
        let message_ids: Tree<Vec<u8>> = db.open_bincode_tree("chat_message_ids").unwrap();

        let chat_user = ChatUser {
            overview,
            messages,
            message_ids,
        };

        // get chat state for writing
        let mut chat = CHAT.get().write().unwrap();

        // add user to state
        chat.db_ref.insert(user_id.to_bytes(), chat_user.clone());

        // return structure
        chat_user
    }

    /// Process incoming RPC request messages for chat module
    pub fn rpc(
        data: Vec<u8>,
        user_id: Vec<u8>,
        _lan: Option<&mut Lan>,
        _internet: Option<&mut Internet>,
    ) {
        let my_user_id = PeerId::from_bytes(&user_id).unwrap();

        match rpc_proto::Chat::decode(&data[..]) {
            Ok(chat) => {
                match chat.message {
                    Some(rpc_proto::chat::Message::OverviewRequest(_overview_request)) => {
                        // get overview list from data base
                        let overview_list = Self::get_overview(my_user_id);

                        // pack message
                        let proto_message = rpc_proto::Chat {
                            message: Some(rpc_proto::chat::Message::OverviewList(overview_list)),
                        };
                        // encode message
                        let mut buf = Vec::with_capacity(proto_message.encoded_len());
                        proto_message
                            .encode(&mut buf)
                            .expect("Vec<u8> provides capacity as needed");

                        // send message
                        Rpc::send_message(
                            buf,
                            crate::rpc::proto::Modules::Chat.into(),
                            "".to_string(),
                            Vec::new(),
                        );
                    }
                    Some(rpc_proto::chat::Message::ConversationRequest(conversation_request)) => {
                        // get messages of a conversation from data base
                        let conversation_list =
                            Self::get_messages(my_user_id, conversation_request.conversation_id);

                        // pack message
                        let proto_message = rpc_proto::Chat {
                            message: Some(rpc_proto::chat::Message::ConversationList(
                                conversation_list,
                            )),
                        };

                        // encode message
                        let mut buf = Vec::with_capacity(proto_message.encoded_len());
                        proto_message
                            .encode(&mut buf)
                            .expect("Vec<u8> provides capacity as needed");
                        Rpc::send_message(
                            buf,
                            crate::rpc::proto::Modules::Chat.into(),
                            "".to_string(),
                            Vec::new(),
                        );
                        // send messageproto::Container, "".to_string(), Vec::new() );
                    }
                    Some(rpc_proto::chat::Message::ChatGroupRequest(conversation_request)) => {
                        // get messages of a conversation from data base
                        let conversation_list =
                            Self::get_messages(my_user_id, conversation_request.group_id);

                        let chat_group_list = rpc_proto::ChatGroupList {
                            group_id: conversation_list.conversation_id.clone(),
                            message_list: conversation_list.message_list,
                        };

                        // pack message
                        let proto_message = rpc_proto::Chat {
                            message: Some(rpc_proto::chat::Message::ChatGroupList(chat_group_list)),
                        };

                        // encode message
                        let mut buf = Vec::with_capacity(proto_message.encoded_len());
                        proto_message
                            .encode(&mut buf)
                            .expect("Vec<u8> provides capacity as needed");

                        // send message
                        Rpc::send_message(
                            buf,
                            crate::rpc::proto::Modules::Chat.into(),
                            "".to_string(),
                            Vec::new(),
                        );
                    }

                    Some(rpc_proto::chat::Message::Send(message)) => {
                        // print message
                        log::info!("sending chat message: {}", message.content.clone());

                        // get user account from user_id
                        let user_account;
                        match PeerId::from_bytes(&user_id) {
                            Ok(user_id_decoded) => match UserAccounts::get_by_id(user_id_decoded) {
                                Some(account) => {
                                    user_account = account;
                                }
                                None => {
                                    log::error!(
                                        "user account id not found: {:?}",
                                        user_id_decoded.to_base58()
                                    );
                                    return;
                                }
                            },
                            Err(e) => {
                                log::error!("user account id could'nt be encoded: {:?}", e);
                                return;
                            }
                        }

                        // send the message
                        if let Err(error) = Self::send_to_peer(&user_account, message.clone()) {
                            log::error!("Outgoing chat message error: {}", error)
                        }
                    }
                    _ => {
                        log::error!("Unhandled Protobuf Chat Message");
                    }
                }
            }
            Err(error) => {
                log::error!("{:?}", error);
            }
        }
    }
}
