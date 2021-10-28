//! Qaul Community Router
//! 
//! This module implements all the tables and logic of the 
//! qaul router.

use prost::Message;

pub mod neighbours;
pub mod users;
pub mod flooder;
pub mod table;
pub mod connections;
pub mod info;

use neighbours::Neighbours;
use users::Users;
use flooder::Flooder;
use table::RoutingTable;
use connections::ConnectionTable;
use info::RouterInfo;

/// Import protobuf message definition generated by 
/// the rust module prost-build.
pub mod proto { include!("qaul.rpc.router.rs"); }


/// qaul community router access
pub struct Router {

}

impl Router {
    /// Initialize the qaul router
    pub fn init() {
        // initialize direct neighbours table
        Neighbours::init();

        // initialize users table
        Users::init();

        // initialize flooder queue
        Flooder::init();

        // initialize the global routing table
        RoutingTable::init();

        // initialize the routing information collection
        // tables per connection module
        ConnectionTable::init();

        // initialize RouterInfo submodule that 
        // scheduals the sending of the routing information
        // to the neighbouring nodes.
        RouterInfo::init(10);
    }

    /// Process commandline instructions for the router 
    /// module and forward them to the submodules
    pub fn cli(cmd: &str) {
        match cmd {
            // connections
            cmd if cmd.starts_with("connections ") => {
                ConnectionTable::cli(cmd.strip_prefix("connections ").unwrap());
            },
            // info
            cmd if cmd.starts_with("info ") => {
                RouterInfo::cli(cmd.strip_prefix("info ").unwrap());
            },
            // neighbours
            cmd if cmd.starts_with("neighbours ") => {
                Neighbours::cli(cmd.strip_prefix("neighbours ").unwrap());
            },
            // table
            cmd if cmd.starts_with("table ") => {
                RoutingTable::cli(cmd.strip_prefix("table ").unwrap());
            },
            // users
            cmd if cmd.starts_with("users ") => {
                Users::cli(cmd.strip_prefix("users ").unwrap());
            },
            // unhandled command
            _ => log::error!("unknown router command"),
        }
    }

    /// Process incoming RPC request messages and send them to
    /// the submodules
    pub fn rpc(data: Vec<u8>) {
        match proto::Router::decode(&data[..]) {
            Ok(router) => {
                match router.message {
                    Some(proto::router::Message::UserRequest(user_request)) => {
                        // send it to submodule
                        Users::rpc( proto::Router{
                            message: Some(proto::router::Message::UserRequest(
                                user_request)),
                        });
                    },
                    _ => {},
                }
            },
            Err(error) => {
                log::error!("{:?}", error);
            },
        }
    }
}