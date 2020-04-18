//! User profile database wrappers (models)

use super::Conv;
use crate::{
    users::{UserProfile, UserUpdate},
    Identity,
};
use alexandria::{
    record::{kv::Value, Record},
    utils::Diff,
};
use async_std::sync::Arc;
use ed25519_dalek::Keypair;
use std::collections::{BTreeMap, BTreeSet};

const KPAIR: &'static str = "keypair";
const UID: &'static str = "id";
const D_NAME: &'static str = "display_name";
const R_NAME: &'static str = "real_name";
const BIO: &'static str = "bio";
const SERV: &'static str = "services";
const AVI: &'static str = "avatar";

/// An alexandria abstraction to store a local user
pub(crate) struct LocalUser {
    pub(crate) profile: UserProfile,
    pub(crate) keypair: Arc<Keypair>,
}

impl LocalUser {
    /// Create a new empty local user
    pub(crate) fn new(id: Identity, keypair: Arc<Keypair>) -> Self {
        Self {
            profile: UserProfile::new(id),
            keypair,
        }
    }

    /// Generate the initial diff of metadata
    pub(crate) fn meta_diff(&self) -> Diff {
        Diff::map().insert(KPAIR, self.keypair.to_bytes().to_vec())
    }
}

/// Get a UserProfile from a record
impl From<&Record> for UserProfile {
    fn from(rec: &Record) -> Self {
        let kv = rec.kv();

        Self {
            id: Conv::id(kv.get(UID).unwrap()),
            display_name: kv.get(D_NAME).map(|v| Conv::string(v)),
            real_name: kv.get(R_NAME).map(|v| Conv::string(v)),
            bio: kv
                .get(BIO)
                .map(|v| Conv::map(v))
                .unwrap_or_else(|| Default::default()),
            services: kv
                .get(SERV)
                .map(|v| Conv::set(v))
                .unwrap_or_else(|| Default::default()),
            avatar: kv.get(AVI).map(|v| Conv::binvec(v)),
        }
    }
}

impl UserProfile {
    /// Generate the first insert diff based on an empty record
    pub(crate) fn init_diff(&self) -> Vec<Diff> {
        let mut v = vec![Diff::map().insert(UID, self.id.as_bytes().to_vec())];

        if let Some(ref d_name) = self.display_name {
            v.push(Diff::map().insert(D_NAME, d_name.clone()));
        }
        if let Some(ref r_name) = self.real_name {
            v.push(Diff::map().insert(R_NAME, r_name.clone()));
        }

        v.push(
            Diff::map().insert(
                BIO,
                self.bio
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone().into()))
                    .collect::<BTreeMap<String, Value>>(),
            ),
        );
        v.push(
            Diff::map().insert(
                SERV,
                self.services
                    .iter()
                    .map(|k| k.clone().into())
                    .collect::<Vec<Value>>(),
            ),
        );

        if let Some(ref avi) = self.avatar {
            v.push(Diff::map().insert(AVI, avi.clone()));
        }

        v
    }

    /// Diff based on how a `UserUpdate` applies to a `UserProfile`
    pub(crate) fn gen_diff(&self, update: UserUpdate) -> Diff {
        use UserUpdate::*;

        match update {
            // Update data if it was previously set
            DisplayName(Some(name)) if self.display_name.is_some() => {
                Diff::map().update(D_NAME, name)
            }
            RealName(Some(name)) if self.real_name.is_some() => Diff::map().update(R_NAME, name),
            SetBioLine(key, val) if self.bio.contains_key(&key) => {
                Diff::map().nested(D_NAME, Diff::map().update(key, val))
            }
            RemoveBioLine(key) if self.display_name.is_some() => {
                Diff::map().nested(D_NAME, Diff::map().delete(key))
            }
            AddService(service) if self.services.contains(&service) => unimplemented!(),
            RemoveService(service) if self.services.contains(&service) => unimplemented!(),

            // Insert if it wasn't
            DisplayName(Some(name)) => Diff::map().insert(D_NAME, name),
            RealName(Some(name)) => Diff::map().insert(R_NAME, name),
            SetBioLine(key, val) => Diff::map().nested(BIO, Diff::map().insert(key, val)),
            RemoveBioLine(key) => Diff::map().nested(BIO, Diff::map().delete(key)),
            AddService(_) => unimplemented!(),
            RemoveService(_) => unimplemented!(),

            // Delete if set to None
            DisplayName(None) => Diff::map().delete(D_NAME),
            RealName(None) => Diff::map().delete(R_NAME),

            // Avatars are a little special
            AvatarData(Some(data)) => Diff::map().delete(AVI).insert(AVI, data),
            AvatarData(None) => Diff::map().delete(BIO),
        }
    }
}

#[test]
fn persist_user_profile() {
    use crate::Identity;
    use alexandria::{
        utils::{Path, TagSet},
        Builder,
    };

    let dir = tempfile::tempdir().unwrap();
    let lib = Builder::new().offset(dir.path()).build().unwrap();

    let profile = UserProfile {
        id: Identity::random(),
        display_name: Some("spacekookie".into()),
        real_name: Some("Katharina Fey".into()),
        bio: {
            let mut tree = BTreeMap::new();
            tree.insert("location".into(), "The internet".into());
            tree.insert("languages".into(), "en, de, fr, eo, ru".into());
            tree
        },
        services: vec![
            "net.qaul.chat",
            "net.qaul.feed",
            "net.qaul.voice",
            "space.kookie.chess",
        ]
        .into_iter()
        .map(|s| s.into())
        .collect(),
        avatar: None,
    };

    let path = Path::from(format!("/users:{}", profile.id));

    let diffs = profile.init_diff();
    async_std::task::block_on(async {
        lib.data(None)
            .await
            .as_ref()
            .unwrap()
            .batch(path.clone(), TagSet::empty(), diffs)
            .await
    })
    .unwrap();
}
