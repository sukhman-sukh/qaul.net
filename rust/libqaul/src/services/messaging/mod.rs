// Copyright (c) 2021 Open Community Project Association https://ocpa.ch
// This software is published under the AGPLv3 license.

//! # Qaul Messaging Service
//!
//! The messaging service is used for sending, receiving and
//! relay chat messages.

use crate::connections::ConnectionModule;
use libp2p::PeerId;
use prost::Message;
use state::Storage;
use std::collections::VecDeque;
use std::sync::RwLock;

use crate::node::user_accounts::{UserAccount, UserAccounts};
use crate::router;
use crate::router::table::RoutingTable;
use super::chat::Chat;
use super::crypto::Crypto;
use super::filesharing;
use super::group;
use super::rtc;

use crate::storage::database::DataBase;
use crate::utilities::timestamp::Timestamp;
use qaul_messaging::QaulMessagingReceived;
use serde::{Deserialize, Serialize};
use sled_extensions::{bincode::Tree, DbExt};

/// Import protobuf message definition generated by
/// the rust module prost-build.
pub mod proto {
    include!("qaul.net.messaging.rs");
}

/// mutable state of messages, scheduled for sending
pub static MESSAGING: Storage<RwLock<Messaging>> = Storage::new();

/// mutable state of failed messages, scheduled for sending
pub static FAILEDMESSAGING: Storage<RwLock<FailedMessaging>> = Storage::new();

/// Messaging Scheduling Structure
pub struct ScheduledMessage {
    receiver: PeerId,
    container: proto::Container,
}

/// Qaul Messaging Structure
pub struct Messaging {
    /// ring buffer of messages scheduled for sending
    pub to_send: VecDeque<ScheduledMessage>,
}

/// Qaul Failed Message Structure
#[derive(Serialize, Deserialize, Clone)]
pub struct FailedMessage {
    pub user_id: Vec<u8>,
    pub conversation_id: Vec<u8>,
    pub created_at: u64,
    pub last_try: u64,
    pub try_count: u32,
    pub message: String,
}

/// Qaul Failed Messaging Structure
pub struct FailedMessaging {
    pub tree: Tree<FailedMessage>,
}

impl Messaging {
    /// Initialize messaging and create the ring buffer.
    pub fn init() {
        let messaging = Messaging {
            to_send: VecDeque::new(),
        };
        MESSAGING.set(RwLock::new(messaging));

        let db = DataBase::get_node_db();
        let tree: Tree<FailedMessage> = db.open_bincode_tree("failed_messages").unwrap();
        let failed_messaging = FailedMessaging { tree: tree };
        FAILEDMESSAGING.set(RwLock::new(failed_messaging));
    }

    // // create DB key from conversation ID, timestamp
    // fn get_db_key_from_vec(conversation_id: Vec<u8>, timestamp: u64) -> Vec<u8> {
    //     let mut timestamp_bytes = timestamp.to_be_bytes().to_vec();
    //     let mut userid_bytes = timestamp.to_be_bytes().to_vec();        
    //     let mut key_bytes = conversation_id;

    //     userid_bytes.append(&mut key_bytes);
    //     userid_bytes.append(&mut timestamp_bytes);
    //     userid_bytes
    // }

    // /// save failed message 
    // pub fn save_failed_outgoing_message(user_id: PeerId, conversation_id: PeerId, contents: String){
    //     let timestamp = Timestamp::get_timestamp();
    //     let key = Self::get_db_key_from_vec(conversation_id.to_bytes(), timestamp);

    //     // create chat message
    //     let message = FailedMessage {
    //         user_id: user_id.to_bytes(),
    //         conversation_id: conversation_id.to_bytes(),
    //         message: contents,
    //         last_try: timestamp,
    //         created_at: timestamp,
    //         try_count: 1,
    //     };

    //     let failed_meesaging = FAILEDMESSAGING.get().write().unwrap();

    //     // save message in data base
    //     if let Err(e) = failed_meesaging.tree.insert(key, message) {
    //         log::error!("Error saving failed chat message to data base: {}", e);
    //     }

    //     // flush trees to disk
    //     if let Err(e) = failed_meesaging.tree.flush() {
    //         log::error!("Error failed chat messages flush: {}", e);
    //     }        
    // }

    /// pack, sign and schedule a message for sending
    pub fn pack_and_send_message(
        user_account: &UserAccount,
        receiver: PeerId,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        // encrypt data
        // TODO: slize data to 64K
        let (encryption_result, nonce) = Crypto::encrypt(data, user_account.to_owned(), receiver.clone());

        let mut encrypted: Vec<proto::Data> = Vec::new();
        match encryption_result {
            Some(encrypted_chunk) => {
                let data_message = proto::Data{ nonce, data: encrypted_chunk };

                log::info!("data len: {}", data_message.encoded_len());

                encrypted.push(data_message);
            },
            None => return Err("Encryption error occurred".to_string()),
        }

        log::info!("sender_id: {}, receiver_id: {}", user_account.id.to_bytes().len(), receiver.to_bytes().len());

        // create envelope
        let envelope = proto::Envelope {
            sender_id: user_account.id.to_bytes(),
            receiver_id: receiver.to_bytes(),
            data: encrypted,
        };

        // debug
        log::info!("envelope len: {}", envelope.encoded_len());

        // encode envelope
        let mut envelope_buf = Vec::with_capacity(envelope.encoded_len());
        envelope
            .encode(&mut envelope_buf)
            .expect("Vec<u8> provides capacity as needed");

        // sign message
        if let Ok(signature) = user_account.keys.sign(&envelope_buf) {
            // create container
            let container = proto::Container {
                signature: signature.clone(),
                envelope: Some(envelope),
            };

            // schedule message for sending
            Self::schedule_message(receiver, container);

            // return signature
            Ok(signature)
        } else {
            return Err("messaging signing error".to_string());
        }
    }

    /// schedule a message
    ///
    /// schedule a message for sending.
    /// This function adds the message to the ring buffer for sending.
    /// This buffer is checked regularly by libqaul for sending.
    ///
    fn schedule_message(receiver: PeerId, container: proto::Container) {
        let msg = ScheduledMessage {
            receiver,
            container,
        };

        // add it to sending queue
        let mut messaging = MESSAGING.get().write().unwrap();
        messaging.to_send.push_back(msg);
    }

    /// Check Scheduler
    ///
    /// Check if there is a message scheduled for sending.
    ///
    pub fn check_scheduler() -> Option<(PeerId, ConnectionModule, Vec<u8>)> {
        let message_item: Option<ScheduledMessage>;

        // get scheduled messaging buffer
        {
            let mut messaging = MESSAGING.get().write().unwrap();
            message_item = messaging.to_send.pop_front();
        }

        if let Some(message) = message_item {
            // check for route
            if let Some(route) = RoutingTable::get_route_to_user(message.receiver) {
                // create binary message
                let data = message.container.encode_to_vec();

                // return information
                return Some((route.node, route.module, data));
            } else {
                log::trace!("No route found to user {}", message.receiver.to_base58());

                // reschedule if no route is found
                Self::schedule_message(message.receiver, message.container);
            }
        }

        None
    }

    /// TODO: send received confirmation message
    fn _send_confirmation(
        user_id: PeerId,
        sender_id: PeerId,
        message_id: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        if let Some(user) = UserAccounts::get_by_id(user_id) {
            // // create timestamp
            let timestamp = Timestamp::get_timestamp();

            // // pack message
            let send_message = proto::Messaging {
                message: Some(proto::messaging::Message::ConfirmationMessage(
                    proto::Confirmation {
                        message_id: message_id,
                        received_at: timestamp,
                    },
                )),
            };

            // encode chat message
            let mut message_buf = Vec::with_capacity(send_message.encoded_len());
            send_message
                .encode(&mut message_buf)
                .expect("Vec<u8> provides capacity as needed");

            // // send message via messaging
            Self::pack_and_send_message(&user, sender_id, message_buf)
        } else {
            return Err("invalid user_id".to_string());
        }
    }

    /// process received message
    pub fn process_received_message(container: proto::Container) {
        // check if there is a message envelope
        if let Some(envelope) = container.envelope {
            if let Ok(sender_id) = PeerId::from_bytes(&envelope.sender_id) {
                // get sender key
                if let Some(key) = router::users::Users::get_pub_key(&sender_id) {
                    // encode envelope
                    let mut envelope_buf = Vec::with_capacity(envelope.encoded_len());
                    envelope
                        .encode(&mut envelope_buf)
                        .expect("Vec<u8> provides capacity as needed");

                    // verify message
                    let verified = key.verify(&envelope_buf, &container.signature);
                    if verified {
                        // to whom is it sent
                        if let Ok(receiver_id) = PeerId::from_bytes(&envelope.receiver_id) {
                            log::info!("messaging envelope.data len = {}", envelope.data.len());

                            // decrypt data
                            let mut decrypted: Vec<u8> = Vec::new();
                            for data_message in envelope.data {
                                if let Some(mut decrypted_chunk) = Crypto::decrypt(data_message.data, data_message.nonce, receiver_id, sender_id.clone()) {
                                    decrypted.append(&mut decrypted_chunk);
                                }
                                else {
                                    log::error!("decryption error");
                                    return;
                                }
                            }

                            // decode data
                            match proto::Messaging::decode(&decrypted[..]) {
                                Ok(messaging) => {
                                    match messaging.message {
                                        Some(proto::messaging::Message::CryptoService(_crypto_service_message)) => {
                                            // implement crypto re-keying here in the future
                                        },
                                        Some(proto::messaging::Message::ConfirmationMessage(
                                            confirmation,
                                        )) => {
                                            log::error!(
                                                "chat confirmation message: {}",
                                                bs58::encode(confirmation.message_id.clone())
                                                    .into_string()
                                            );
                                            // confirm successful send of chat message
                                            // TODO: send confirmation message
                                            Chat::update_confirmation(
                                                receiver_id,
                                                confirmation.message_id,
                                                confirmation.received_at,
                                            );
                                        },
                                        Some(proto::messaging::Message::ChatMessage(
                                            chat_message,
                                        )) => {
                                            log::error!("chat message: {}", chat_message.content);
                                            // send data to chat
                                            if Chat::save_incoming_chat_message(receiver_id, sender_id, chat_message, container.signature.clone()) == true{
                                                //send confirm message
                                                match Self::_send_confirmation(
                                                    receiver_id,
                                                    sender_id,
                                                    container.signature.clone(),
                                                ) {
                                                    Ok(_) => {
                                                        log::error!("Outgoing chat confirmation message id=: {}", bs58::encode(container.signature.clone()).into_string())
                                                    }
                                                    Err(e) => {
                                                        log::error!(
                                                            "Outgoing chat message error: {}",
                                                            e
                                                        )
                                                    }
                                                }
                                            }
                                        },
                                        Some(proto::messaging::Message::FileMessage(
                                            file_message,
                                        )) => {
                                            filesharing::FileShare::on_receive_message(
                                                sender_id,
                                                receiver_id,
                                                file_message.content,
                                            );
                                        },
                                        Some(proto::messaging::Message::GroupMessage(
                                            group_message,
                                        )) => {
                                            group::Group::net(sender_id, receiver_id, group_message.content, container.signature);
                                        },        
                                        Some(proto::messaging::Message::RtcMessage(
                                            rtc_message,
                                        )) => {
                                            rtc::Rtc::net(sender_id, receiver_id, rtc_message.content, container.signature);
                                        },                                                                         
                                        None => {
                                            log::error!(
                                                "message {} from {} was empty",
                                                bs58::encode(container.signature).into_string(),
                                                sender_id.to_base58()
                                            )
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!(
                                        "Error decoding Messaging Message {} from {} to {}: {}",
                                        bs58::encode(container.signature).into_string(),
                                        sender_id.to_base58(),
                                        receiver_id.to_base58(),
                                        e
                                    );
                                }
                            }
                        } else {
                            log::error!(
                                "receiver ID of message {} from {} not valid",
                                bs58::encode(container.signature).into_string(),
                                sender_id.to_base58()
                            );
                        }
                    } else {
                        log::error!("verification failed");
                    }
                } else {
                    log::error!("No key found for user {}", sender_id.to_base58());
                }
            } else {
                log::error!("Error retrieving PeerId");
            }
        } else {
            log::error!("No Envelope in Message Container");
        }
    }

    /// received message from qaul_messaging behaviour
    pub fn received(received: QaulMessagingReceived) {
        // decode message container
        match proto::Container::decode(&received.data[..]) {
            Ok(container) => {
                if let Some(envelope) = container.envelope.clone() {
                    match PeerId::from_bytes(&envelope.receiver_id) {
                        Ok(receiver_id) => {
                            // check if message is local user account
                            if UserAccounts::is_account(receiver_id) {
                                // save message
                                Self::process_received_message(container);
                            } else {
                                // schedule it for further sending otherwise
                                Self::schedule_message(receiver_id, container);
                            }
                        }
                        Err(e) => log::error!(
                            "invalid peer ID of message {}: {}",
                            bs58::encode(container.signature).into_string(),
                            e
                        ),
                    }
                }
            }
            Err(e) => log::error!("Messaging container decoding error: {}", e),
        }
    }
}
