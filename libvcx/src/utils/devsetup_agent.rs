
pub const SERIALIZE_VERSION: &'static str = "2.0";

#[cfg(test)]
pub mod test {
    use indy_sys::WalletHandle;

    use agency_client::payload::PayloadKinds;

    use crate::{aries, connection, credential, credential_def, disclosed_proof, issuer_credential, libindy, proof, schema, settings, utils};
    use crate::aries::messages::a2a::A2AMessage;
    use crate::error::{VcxError, VcxErrorKind, VcxResult};
    use crate::libindy::utils::wallet::*;
    use crate::utils::devsetup::*;
    use crate::utils::plugins::init_plugin;
    use crate::utils::provision::{provision_cloud_agent, ProvisionAgentConfig, AgencyConfig};
    use crate::init::{open_as_main_wallet, init_issuer_config, create_agency_client_for_main_wallet, PoolConfig};
    use crate::utils::constants;

    #[derive(Debug)]
    pub struct VcxAgencyMessage {
        pub uid: String,
        pub decrypted_msg: String,
    }

    fn determine_message_type(a2a_message: A2AMessage) -> PayloadKinds {
        debug!("determine_message_type >>> a2a_message={:?}", a2a_message);
        match a2a_message.clone() {
            A2AMessage::PresentationRequest(_) => PayloadKinds::ProofRequest,
            A2AMessage::CredentialOffer(_) => PayloadKinds::CredOffer,
            A2AMessage::Credential(_) => PayloadKinds::Cred,
            A2AMessage::Presentation(_) => PayloadKinds::Proof,
            _msg => PayloadKinds::Other(String::from("aries"))
        }
    }

    fn str_message_to_a2a_message(message: &str) -> VcxResult<A2AMessage> {
        Ok(serde_json::from_str(message)
            .map_err(|err| VcxError::from_msg(VcxErrorKind::InvalidJson, format!("Cannot deserialize A2A message: {}", err)))?
        )
    }

    fn str_message_to_payload_type(message: &str) -> VcxResult<PayloadKinds> {
        let a2a_message = str_message_to_a2a_message(message)?;
        Ok(determine_message_type(a2a_message))
    }

    fn download_message(did: String, filter_msg_type: PayloadKinds) -> VcxAgencyMessage {
        let mut messages = agency_client::get_message::download_messages_noauth(Some(vec![did]), Some(vec![String::from("MS-103")]), None).unwrap();
        assert_eq!(1, messages.len());
        let messages = messages.pop().unwrap();

        for message in messages.msgs.into_iter() {
            let decrypted_msg = &message.decrypted_msg.unwrap();
            let msg_type = str_message_to_payload_type(decrypted_msg).unwrap();
            if filter_msg_type == msg_type {
                return VcxAgencyMessage {
                    uid: message.uid,
                    decrypted_msg: decrypted_msg.clone(),
                };
            }
        }
        panic!("Message not found")
    }

    pub trait TestAgent {
        fn activate(&self);
    }

    pub struct Faber {
        pub config_wallet: WalletConfig,
        pub config_agency: AgencyConfig,
        pub config_issuer: IssuerConfig,
        pub wallet_handle: WalletHandle,
        pub config: String,
        pub connection_handle: u32,
        pub schema_handle: u32,
        pub cred_def_handle: u32,
        pub credential_handle: u32,
        pub presentation_handle: u32,
    }

    impl TestAgent for Faber {
        fn activate(&self) {
            utils::devsetup::set_new_config(&self.config);
        }
    }

    impl TestAgent for Alice {
        fn activate(&self) {
            utils::devsetup::set_new_config(&self.config);
        }
    }

    impl Faber {
        pub fn setup() -> Faber {
            settings::clear_config();
            init_test_logging();
            let enterprise_seed = "000000000000000000000000Trustee1";

            let config_wallet = WalletConfig {
                wallet_name: format!("faber_wallet_{}", uuid::Uuid::new_v4().to_string()),
                wallet_key: settings::DEFAULT_WALLET_KEY.into(),
                wallet_key_derivation: settings::WALLET_KDF_RAW.into(),
                wallet_type: None,
                storage_config: None,
                storage_credentials: None,
                rekey: None,
                rekey_derivation_method: None
            };

            let config_provision_agent = ProvisionAgentConfig {
                agency_did: AGENCY_DID.to_string(),
                agency_verkey: AGENCY_VERKEY.to_string(),
                agency_endpoint: AGENCY_ENDPOINT.to_string(),
                agent_seed: None
            };

            let config_pool = PoolConfig {
                genesis_path: constants::GENESIS_PATH.to_string(),
                pool_name: Some(constants::POOL.to_string()),
                pool_config: None
            };

            create_wallet(&config_wallet).unwrap();
            let wallet_handle = open_as_main_wallet(&config_wallet).unwrap();
            let config_issuer = configure_issuer_wallet(enterprise_seed).unwrap();
            init_issuer_config(&config_issuer); // todo: this line can be removed probably
            let config_agency = provision_cloud_agent(&config_provision_agent).unwrap();

            let config = combine_configs(&config_wallet, &config_agency, Some(&config_issuer), wallet_handle);

            Faber {
                config,
                config_wallet,
                config_agency,
                config_issuer,
                schema_handle: 0,
                cred_def_handle: 0,
                connection_handle: 0,
                wallet_handle: get_wallet_handle(),
                credential_handle: 0,
                presentation_handle: 0,
            }
        }

        pub fn activate(&self) {
            info!("faber activate >>> going to clear config");
            settings::clear_config();
            // todo: Would be nicer to load library state bit more explicitly than just "blindly" loading dumped state
            // init_issuer_config(&self.config_issuer);
            // create_agency_client_for_main_wallet(&self.config_agency);
            info!("faber activate >>> going to process config string: {}", &self.config);
            let res = settings::process_config_string(&self.config, false);
            warn!("process config res = {:?}", res);
            info!("faber activate >>> going to set wallet handle");
            set_wallet_handle(self.wallet_handle);
        }

        pub fn create_schema(&mut self) {
            self.activate();
            let did = String::from("V4SGRU86Z58d6TV7PBUe6f");
            let data = r#"["name","date","degree", "empty_param"]"#.to_string();
            let name: String = crate::utils::random::generate_random_schema_name();
            let version: String = String::from("1.0");

            self.schema_handle = schema::create_and_publish_schema("test_schema", did.clone(), name, version, data).unwrap();
        }

        pub fn create_credential_definition(&mut self) {
            self.activate();

            let schema_id = schema::get_schema_id(self.schema_handle).unwrap();
            let did = String::from("V4SGRU86Z58d6TV7PBUe6f");
            let name = String::from("degree");
            let tag = String::from("tag");

            self.cred_def_handle = credential_def::create_and_publish_credentialdef(String::from("test_cred_def"), name, did.clone(), schema_id, tag, String::from("{}")).unwrap();
        }

        pub fn create_presentation_request(&self) -> u32 {
            let requested_attrs = json!([
                {"name": "name"},
                {"name": "date"},
                {"name": "degree"},
                {"name": "empty_param", "restrictions": {"attr::empty_param::value": ""}}
            ]).to_string();

            proof::create_proof(String::from("alice_degree"),
                                requested_attrs,
                                json!([]).to_string(),
                                json!({}).to_string(),
                                String::from("proof_from_alice")).unwrap()
        }

        pub fn create_invite(&mut self) -> String {
            self.activate();
            self.connection_handle = connection::create_connection("alice").unwrap();
            connection::connect(self.connection_handle).unwrap();
            connection::update_state(self.connection_handle).unwrap();
            assert_eq!(2, connection::get_state(self.connection_handle));

            connection::get_invite_details(self.connection_handle).unwrap()
        }

        pub fn update_state(&self, expected_state: u32) {
            self.activate();
            connection::update_state(self.connection_handle).unwrap();
            assert_eq!(expected_state, connection::get_state(self.connection_handle));
        }

        pub fn ping(&self) {
            self.activate();
            connection::send_ping(self.connection_handle, None).unwrap();
        }

        pub fn discovery_features(&self) {
            self.activate();
            connection::send_discovery_features(self.connection_handle, None, None).unwrap();
        }

        pub fn connection_info(&self) -> serde_json::Value {
            self.activate();
            let details = connection::get_connection_info(self.connection_handle).unwrap();
            serde_json::from_str(&details).unwrap()
        }

        pub fn offer_credential(&mut self) {
            self.activate();

            let did = String::from("V4SGRU86Z58d6TV7PBUe6f");
            let credential_data = json!({
                "name": "alice",
                "date": "05-2018",
                "degree": "maths",
                "empty_param": ""
            }).to_string();

            self.credential_handle = issuer_credential::issuer_credential_create(self.cred_def_handle,
                                                                                 String::from("alice_degree"),
                                                                                 did,
                                                                                 String::from("cred"),
                                                                                 credential_data,
                                                                                 0).unwrap();
            issuer_credential::send_credential_offer(self.credential_handle, self.connection_handle, None).unwrap();
            issuer_credential::update_state(self.credential_handle, None, self.connection_handle).unwrap();
            assert_eq!(2, issuer_credential::get_state(self.credential_handle).unwrap());
        }

        pub fn send_credential(&self) {
            self.activate();
            issuer_credential::update_state(self.credential_handle, None, self.connection_handle).unwrap();
            assert_eq!(3, issuer_credential::get_state(self.credential_handle).unwrap());

            issuer_credential::send_credential(self.credential_handle, self.connection_handle).unwrap();
            issuer_credential::update_state(self.credential_handle, None, self.connection_handle).unwrap();
            assert_eq!(4, issuer_credential::get_state(self.credential_handle).unwrap());
            assert_eq!(aries::messages::status::Status::Success.code(), issuer_credential::get_credential_status(self.credential_handle).unwrap());
        }

        pub fn request_presentation(&mut self) {
            self.activate();
            self.presentation_handle = self.create_presentation_request();
            assert_eq!(1, proof::get_state(self.presentation_handle).unwrap());

            proof::send_proof_request(self.presentation_handle, self.connection_handle, None).unwrap();
            proof::update_state(self.presentation_handle, None, self.connection_handle).unwrap();

            assert_eq!(2, proof::get_state(self.presentation_handle).unwrap());
        }

        pub fn verify_presentation(&self) {
            self.activate();
            self.update_proof_state(4, aries::messages::status::Status::Success.code())
        }

        pub fn update_proof_state(&self, expected_state: u32, expected_status: u32) {
            self.activate();

            proof::update_state(self.presentation_handle, None, self.connection_handle).unwrap();
            assert_eq!(expected_state, proof::get_state(self.presentation_handle).unwrap());
            assert_eq!(expected_status, proof::get_proof_state(self.presentation_handle).unwrap());
        }

        pub fn teardown(&self) {
            self.activate();
            close_main_wallet().unwrap();
            delete_wallet(&self.config_wallet.wallet_name, &self.config_wallet.wallet_key, &self.config_wallet.wallet_key_derivation, None, None, None).unwrap();
        }
    }

    pub struct Alice {
        pub config_wallet: WalletConfig,
        pub config_agency: AgencyConfig,
        pub wallet_handle: WalletHandle,
        pub config: String,
        pub connection_handle: u32,
        pub credential_handle: u32,
        pub presentation_handle: u32,
    }

    impl Alice {
        pub fn setup() -> Alice {
            settings::clear_config();
            init_test_logging();

            let config_wallet = WalletConfig {
                wallet_name: format!("alice_wallet_{}", uuid::Uuid::new_v4().to_string()),
                wallet_key: settings::DEFAULT_WALLET_KEY.into(),
                wallet_key_derivation: settings::WALLET_KDF_RAW.into(),
                wallet_type: None,
                storage_config: None,
                storage_credentials: None,
                rekey: None,
                rekey_derivation_method: None
            };

            let config_provision_agent = ProvisionAgentConfig {
                agency_did: C_AGENCY_DID.to_string(),
                agency_verkey: C_AGENCY_VERKEY.to_string(),
                agency_endpoint: C_AGENCY_ENDPOINT.to_string(),
                agent_seed: None
            };

            create_wallet(&config_wallet).unwrap();
            let wallet_handle = open_as_main_wallet(&config_wallet).unwrap();
            let config_agency = provision_cloud_agent(&config_provision_agent).unwrap();

            let config = combine_configs(&config_wallet, &config_agency, None, wallet_handle);

            Alice {
                config,
                config_wallet,
                config_agency,
                wallet_handle: get_wallet_handle(),
                connection_handle: 0,
                credential_handle: 0,
                presentation_handle: 0,
            }
        }

        pub fn activate(&self) {
            settings::clear_config();
            settings::process_config_string(&self.config, false).unwrap();
            set_wallet_handle(self.wallet_handle);
        }

        pub fn accept_invite(&mut self, invite: &str) {
            self.activate();
            self.connection_handle = connection::create_connection_with_invite("faber", invite).unwrap();
            connection::connect(self.connection_handle).unwrap();
            connection::update_state(self.connection_handle).unwrap();
            assert_eq!(3, connection::get_state(self.connection_handle));
        }

        pub fn update_state(&self, expected_state: u32) {
            self.activate();
            connection::update_state(self.connection_handle).unwrap();
            assert_eq!(expected_state, connection::get_state(self.connection_handle));
        }

        pub fn download_message(&self, message_type: PayloadKinds) -> VcxAgencyMessage {
            self.activate();
            let did = connection::get_pw_did(self.connection_handle).unwrap();
            download_message(did, message_type) // tood: need to pass PayloadKind
        }

        pub fn accept_offer(&mut self) {
            self.activate();
            let offers = credential::get_credential_offer_messages(self.connection_handle).unwrap();
            let offer = serde_json::from_str::<Vec<::serde_json::Value>>(&offers).unwrap()[0].clone();
            let offer_json = serde_json::to_string(&offer).unwrap();

            self.credential_handle = credential::credential_create_with_offer("degree", &offer_json).unwrap();
            assert_eq!(3, credential::get_state(self.credential_handle).unwrap());

            credential::send_credential_request(self.credential_handle, self.connection_handle).unwrap();
            assert_eq!(2, credential::get_state(self.credential_handle).unwrap());
        }

        pub fn accept_credential(&self) {
            self.activate();
            credential::update_state(self.credential_handle, None, self.connection_handle).unwrap();
            assert_eq!(4, credential::get_state(self.credential_handle).unwrap());
            assert_eq!(aries::messages::status::Status::Success.code(), credential::get_credential_status(self.credential_handle).unwrap());
        }

        pub fn get_proof_request_messages(&self) -> String {
            self.activate();
            let presentation_requests = disclosed_proof::get_proof_request_messages(self.connection_handle).unwrap();
            let presentation_request = serde_json::from_str::<Vec<::serde_json::Value>>(&presentation_requests).unwrap()[0].clone();
            let presentation_request_json = serde_json::to_string(&presentation_request).unwrap();
            presentation_request_json
        }

        pub fn get_credentials_for_presentation(&self) -> serde_json::Value {
            let credentials = disclosed_proof::retrieve_credentials(self.presentation_handle).unwrap();
            let credentials: std::collections::HashMap<String, serde_json::Value> = serde_json::from_str(&credentials).unwrap();

            let mut use_credentials = json!({});

            for (referent, credentials) in credentials["attrs"].as_object().unwrap().iter() {
                use_credentials["attrs"][referent] = json!({
                    "credential": credentials[0]
                })
            }

            use_credentials
        }

        pub fn send_presentation(&mut self) {
            self.activate();
            let presentation_request_json = self.get_proof_request_messages();

            self.presentation_handle = disclosed_proof::create_proof("degree", &presentation_request_json).unwrap();

            let credentials = self.get_credentials_for_presentation();

            disclosed_proof::generate_proof(self.presentation_handle, credentials.to_string(), String::from("{}")).unwrap();
            assert_eq!(3, disclosed_proof::get_state(self.presentation_handle).unwrap());

            disclosed_proof::send_proof(self.presentation_handle, self.connection_handle).unwrap();
            assert_eq!(2, disclosed_proof::get_state(self.presentation_handle).unwrap());
        }

        pub fn decline_presentation_request(&mut self) {
            self.activate();
            let presentation_request_json = self.get_proof_request_messages();

            self.presentation_handle = disclosed_proof::create_proof("degree", &presentation_request_json).unwrap();
            disclosed_proof::decline_presentation_request(self.presentation_handle, self.connection_handle, Some(String::from("reason")), None).unwrap();
        }

        pub fn propose_presentation(&mut self) {
            self.activate();
            let presentation_request_json = self.get_proof_request_messages();

            self.presentation_handle = disclosed_proof::create_proof("degree", &presentation_request_json).unwrap();
            let proposal_data = json!({
                "attributes": [
                    {
                        "name": "first name"
                    }
                ],
                "predicates": [
                    {
                        "name": "age",
                        "predicate": ">",
                        "threshold": 18
                    }
                ]
            });
            disclosed_proof::decline_presentation_request(self.presentation_handle, self.connection_handle, None, Some(proposal_data.to_string())).unwrap();
        }

        pub fn ensure_presentation_verified(&self) {
            self.activate();
            disclosed_proof::update_state(self.presentation_handle, None, self.connection_handle).unwrap();
            assert_eq!(aries::messages::status::Status::Success.code(), disclosed_proof::get_presentation_status(self.presentation_handle).unwrap());
        }
    }

    impl Drop for Faber {
        fn drop(&mut self) {
            self.activate();
            close_main_wallet().unwrap();
            delete_wallet(&self.config_wallet.wallet_name, &self.config_wallet.wallet_key, &self.config_wallet.wallet_key_derivation, None, None, None).unwrap();
        }
    }

    impl Drop for Alice {
        fn drop(&mut self) {
            self.activate();
            close_main_wallet().unwrap();
            delete_wallet(&self.config_wallet.wallet_name, &self.config_wallet.wallet_key, &self.config_wallet.wallet_key_derivation, None, None, None).unwrap();
        }
    }
}