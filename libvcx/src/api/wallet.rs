use std::ptr::null;
use std::thread;

use indy::{CommandHandle, SearchHandle, WalletHandle};
use libc::c_char;

use crate::error::prelude::*;
use crate::libindy::utils::payments::{create_address, get_wallet_token_info, pay_a_payee, sign_with_address, verify_with_address};
use crate::libindy::utils::wallet;
use crate::libindy::utils::wallet::{export_main_wallet, import};
use crate::utils;
use crate::utils::cstring::CStringUtils;
use crate::utils::error;
use crate::utils::threadpool::spawn;

/// Creates new wallet and master secret using provided config. Keeps wallet closed.
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// wallet_config: wallet configuration
///
/// cb: Callback that provides configuration or error status
///
/// # Example wallet config ->
/// {
///   "wallet_name": "my_wallet_name",
///   "wallet_key": "123456",
///   "wallet_key_derivation": "ARGON2I_MOD",
///   "wallet_type": "postgres_storage",
///   "storage_config": "{\"url\":\"localhost:5432\"}",
///   "storage_credentials": "{\"account\":\"postgres\",\"password\":\"password_123\",\"admin_account\":\"postgres\",\"admin_password\":\"password_foo\"}"
/// }
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_create_wallet(command_handle: CommandHandle,
                                        wallet_config: *const c_char,
                                        cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32)>) -> u32 {
    info!("vcx_create_wallet >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    check_useful_c_str!(wallet_config, VcxErrorKind::InvalidOption);

    trace!("vcx_create_wallet(command_handle: {}, wallet_config: {})",
           command_handle, wallet_config);

    thread::spawn(move || {
        match wallet::create_wallet_from_config(&wallet_config) {
            Err(e) => {
                error!("vcx_create_wallet_cb(command_handle: {}, rc: {}", command_handle, e);
                cb(command_handle, e.into());
            }
            Ok(_) => {
                trace!("vcx_create_wallet_cb(command_handle: {}, rc: {})",
                       command_handle, error::SUCCESS.message);
                cb(command_handle, 0);
            }
        }
    });

    error::SUCCESS.code_num
}

/// Creates issuer's did and keypair and stores them in the wallet.
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// enterprise_seed: Seed used to generate institution did, keypair and other secrets
///
/// cb: Callback that provides institution config or error status
///
/// # Example institution config ->{
///   "institution_did": "V4SGRU86Z58d6TV7PBUe6f",
///   "institution_verkey": "GJ1SzoWzavQYfNL9XkaJdrQejfztN4XqdsiV4ct3LXKL",
/// }
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_configure_issuer_wallet(command_handle: CommandHandle,
                                        enterprise_seed: *const c_char,
                                        cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32, *const c_char)>) -> u32 {
    info!("vcx_configure_issuer_wallet >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    check_useful_c_str!(enterprise_seed, VcxErrorKind::InvalidOption);

    trace!("vcx_configure_issuer_wallet(command_handle: {}, enterprise_seed: {})",
           command_handle, enterprise_seed);

    thread::spawn(move || {
        match wallet::configure_issuer_wallet(&enterprise_seed) {
            Err(e) => {
                error!("vcx_configure_issuer_wallet_cb(command_handle: {}, rc: {}", command_handle, e);
                cb(command_handle, e.into(), null());
            }
            Ok(conf) => {
                trace!("vcx_configure_issuer_wallet_cb(command_handle: {}, rc: {}, conf: {})",
                       command_handle, error::SUCCESS.message, conf);
                let conf = CStringUtils::string_to_cstring(conf.to_string());
                cb(command_handle, 0, conf.as_ptr());
            }
        }
    });

    error::SUCCESS.code_num
}

/// Opens wallet chosen using provided config and returns its wallet handle.
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// wallet_config: wallet configuration
///
/// cb: Callback that provides wallet handle as u32 (wrappers require unsigned integer) or error status
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_open_main_wallet(command_handle: CommandHandle,
                                        wallet_config: *const c_char,
                                        cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32, wh: u32)>) -> u32 {
    info!("vcx_open_main_wallet >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    check_useful_c_str!(wallet_config, VcxErrorKind::InvalidOption);

    trace!("vcx_open_main_wallet(command_handle: {})", command_handle);

    thread::spawn(move || {
        match wallet::open_wallet_directly(&wallet_config) {
            Err(e) => {
                error!("vcx_open_main_wallet_cb(command_handle: {}, rc: {}", command_handle, e);
                cb(command_handle, e.into(), indy::INVALID_WALLET_HANDLE.0 as u32);
            }
            Ok(wh) => {
                trace!("vcx_open_main_wallet_cb(command_handle: {}, rc: {}, wh: {})",
                       command_handle, error::SUCCESS.message, wh.0);
                cb(command_handle, 0, wh.0 as u32);
            }
        }
    });

    error::SUCCESS.code_num
}

/// Closes wallet chosen using provided wallet handle.
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// wallet_handle: wallet handle as u32 (wrappers require unsigned integer)
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_close_main_wallet(command_handle: CommandHandle,
                                        wallet_handle: u32,
                                        cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32)>) -> u32 {
    info!("vcx_close_main_wallet >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);

    trace!("vcx_close_main_wallet(command_handle: {}, wallet_handle: {})", command_handle, wallet_handle);

    thread::spawn(move || {
        match wallet::close_wallet_directly(indy::WalletHandle(wallet_handle as i32)) {
            Err(e) => {
                error!("vcx_close_main_wallet_cb(command_handle: {}, rc: {}", command_handle, e);
                cb(command_handle, e.into());
            }
            Ok(_) => {
                trace!("vcx_close_main_wallet_cb(command_handle: {}, rc: {})",
                       command_handle, error::SUCCESS.message);
                cb(command_handle, 0);
            }
        }
    });

    error::SUCCESS.code_num
}


/// Get the total balance from all addresses contained in the configured wallet
///
/// #Params
///
/// command_handle: command handle to map callback to user context.
///
/// payment_handle: for future use
///
/// cb: Callback that provides wallet balance
///
/// # Example info -> "{"balance":6,"balance_str":"6","addresses":[{"address":"pay:null:9UFgyjuJxi1i1HD","balance":3,"utxo":[{"source":"pay:null:1","paymentAddress":"pay:null:zR3GN9lfbCVtHjp","amount":1,"extra":"yqeiv5SisTeUGkw"}]}]}"
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_wallet_get_token_info(command_handle: CommandHandle,
                                        payment_handle: u32,
                                        cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32, *const c_char)>) -> u32 {
    info!("vcx_wallet_get_token_info >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    trace!("vcx_wallet_get_token_info(command_handle: {}, payment_handle: {})",
           command_handle, payment_handle);

    spawn(move || {
        match get_wallet_token_info() {
            Ok(x) => {
                trace!("vcx_wallet_get_token_info_cb(command_handle: {}, rc: {}, info: {})",
                       command_handle, 0, x);

                let msg = CStringUtils::string_to_cstring(x.to_string());
                cb(command_handle, error::SUCCESS.code_num, msg.as_ptr());
            }
            Err(x) => {
                warn!("vcx_wallet_get_token_info_cb(command_handle: {}, rc: {}, info: {})",
                      command_handle, x, "null");

                let msg = CStringUtils::string_to_cstring("".to_string());
                cb(command_handle, x.into(), msg.as_ptr());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Add a payment address to the wallet
///
/// #params
///
/// cb: Callback that provides payment address info
///
/// # Example payment_address -> "pay:null:9UFgyjuJxi1i1HD"
///
/// #Returns
/// Error code as u32
#[no_mangle]
pub extern fn vcx_wallet_create_payment_address(command_handle: CommandHandle,
                                                seed: *const c_char,
                                                cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32, address: *const c_char)>) -> u32 {
    info!("vcx_wallet_create_payment_address >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    let seed = if !seed.is_null() {
        check_useful_opt_c_str!(seed, VcxErrorKind::InvalidOption);
        seed
    } else {
        None
    };

    trace!("vcx_wallet_create_payment_address(command_handle: {})",
           command_handle);

    spawn(move || {
        match create_address(seed) {
            Ok(x) => {
                trace!("vcx_wallet_create_payment_address_cb(command_handle: {}, rc: {}, address: {})",
                       command_handle, error::SUCCESS.message, x);

                let msg = CStringUtils::string_to_cstring(x);
                cb(command_handle, error::SUCCESS.code_num, msg.as_ptr());
            }
            Err(x) => {
                warn!("vcx_wallet_create_payment_address_cb(command_handle: {}, rc: {}, address: {})",
                      command_handle, x, "null");

                let msg = CStringUtils::string_to_cstring("".to_string());
                cb(command_handle, x.into(), msg.as_ptr());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}


/// Signs a message with a payment address.
///
/// # Params:
/// command_handle: command handle to map callback to user context.
/// payment_address: payment address of message signer. The key must be created by calling vcx_wallet_create_address
/// message_raw: a pointer to first byte of message to be signed
/// message_len: a message length
/// cb: Callback that takes command result as parameter.
///
/// # Returns:
/// a signature string
#[no_mangle]
pub extern fn vcx_wallet_sign_with_address(command_handle: CommandHandle,
                                           payment_address: *const c_char,
                                           message_raw: *const u8,
                                           message_len: u32,
                                           cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32,
                                                                signature: *const u8, signature_len: u32)>) -> u32 {
    info!("vcx_wallet_sign_with_address >>>");
    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    check_useful_c_str!(payment_address, VcxErrorKind::InvalidOption);
    check_useful_c_byte_array!(message_raw, message_len, VcxErrorKind::InvalidOption, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_sign_with_address(command_handle: {}, payment_address: {}, message_raw: {:?})",
           command_handle, payment_address, message_raw);

    spawn(move || {
        match sign_with_address(&payment_address, message_raw.as_slice()) {
            Ok(signature) => {
                trace!("vcx_wallet_sign_with_address_cb(command_handle: {}, rc: {}, signature: {:?})",
                       command_handle, error::SUCCESS.message, signature);

                let (signature_raw, signature_len) = utils::cstring::vec_to_pointer(&signature);

                cb(command_handle, error::SUCCESS.code_num, signature_raw, signature_len);
            }
            Err(error) => {
                warn!("vcx_wallet_sign_with_address_cb(command_handle: {}, error: {})",
                      command_handle, error);

                cb(command_handle, error.into(), null(), 0);
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}


/// Verify a signature with a payment address.
///
/// #Params
/// command_handle: command handle to map callback to user context.
/// payment_address: payment address of the message signer
/// message_raw: a pointer to first byte of message that has been signed
/// message_len: a message length
/// signature_raw: a pointer to first byte of signature to be verified
/// signature_len: a signature length
/// cb: Callback that takes command result as parameter.
///
/// #Returns
/// valid: true - if signature is valid, false - otherwise
#[no_mangle]
pub extern fn vcx_wallet_verify_with_address(command_handle: CommandHandle,
                                             payment_address: *const c_char,
                                             message_raw: *const u8,
                                             message_len: u32,
                                             signature_raw: *const u8,
                                             signature_len: u32,
                                             cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32,
                                                                  result: bool)>) -> u32 {
    info!("vcx_wallet_sign_with_address >>>");
    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    check_useful_c_str!(payment_address, VcxErrorKind::InvalidOption);
    check_useful_c_byte_array!(message_raw, message_len, VcxErrorKind::InvalidOption, VcxErrorKind::InvalidOption);
    check_useful_c_byte_array!(signature_raw, signature_len, VcxErrorKind::InvalidOption, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_verify_with_address(command_handle: {}, payment_address: {}, message_raw: {:?}, signature_raw: {:?})",
           command_handle, payment_address, message_raw, signature_raw);

    spawn(move || {
        match verify_with_address(&payment_address, message_raw.as_slice(), signature_raw.as_slice()) {
            Ok(valid) => {
                trace!("vcx_wallet_verify_with_address_cb(command_handle: {}, rc: {}, valid: {})",
                       command_handle, error::SUCCESS.message, valid);

                cb(command_handle, error::SUCCESS.code_num, valid);
            }
            Err(error) => {
                warn!("vcx_wallet_verify_with_address_cb(command_handle: {}, error: {})",
                      command_handle, error);

                cb(command_handle, error.into(), false);
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Adds a record to the wallet
/// Assumes there is an open wallet.
/// #Params
///
/// command_handle: command handle to map callback to user context.
///
/// type_: type of record. (e.g. 'data', 'string', 'foobar', 'image')
///
/// id: the id ("key") of the record.
///
/// value: value of the record with the associated id.
///
/// tags_json: the record tags used for search and storing meta information as json:
///   {
///     "tagName1": <str>, // string tag (will be stored encrypted)
///     "tagName2": <int>, // int tag (will be stored encrypted)
///     "~tagName3": <str>, // string tag (will be stored un-encrypted)
///     "~tagName4": <int>, // int tag (will be stored un-encrypted)
///   }
///  The tags_json must be valid json, and if no tags are to be associated with the
/// record, then the empty '{}' json must be passed.
///
/// cb: Callback that any errors or a receipt of transfer
///
/// #Returns
/// Error code as a u32
///
#[no_mangle]
pub extern fn vcx_wallet_add_record(command_handle: CommandHandle,
                                    type_: *const c_char,
                                    id: *const c_char,
                                    value: *const c_char,
                                    tags_json: *const c_char,
                                    cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32)>) -> u32 {
    info!("vcx_wallet_add_record >>>");

    check_useful_c_str!(type_, VcxErrorKind::InvalidOption);
    check_useful_c_str!(id, VcxErrorKind::InvalidOption);
    check_useful_c_str!(value, VcxErrorKind::InvalidOption);
    check_useful_c_str!(tags_json, VcxErrorKind::InvalidOption);
    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_add_record(command_handle: {}, type_: {}, id: {}, value: {}, tags_json: {})",
           command_handle, secret!(&type_), secret!(&id), secret!(&value), secret!(&tags_json));

    spawn(move || {
        match wallet::add_record(&type_, &id, &value, Some(&tags_json)) {
            Ok(()) => {
                trace!("vcx_wallet_add_record(command_handle: {}, rc: {})",
                       command_handle, error::SUCCESS.message);

                cb(command_handle, error::SUCCESS.code_num);
            }
            Err(x) => {
                trace!("vcx_wallet_add_record(command_handle: {}, rc: {})",
                       command_handle, x);

                cb(command_handle, x.into());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Updates the value of a record already in the wallet.
/// Assumes there is an open wallet and that a type and id pair already exists.
/// #Params
///
/// command_handle: command handle to map callback to user context.
///
/// type_: type of record. (e.g. 'data', 'string', 'foobar', 'image')
///
/// id: the id ("key") of the record.
///
/// value: New value of the record with the associated id.
///
/// cb: Callback that any errors or a receipt of transfer
///
/// #Returns
/// Error code as a u32
///
#[no_mangle]
pub extern fn vcx_wallet_update_record_value(command_handle: CommandHandle,
                                             type_: *const c_char,
                                             id: *const c_char,
                                             value: *const c_char,
                                             cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32)>) -> u32 {
    info!("vcx_wallet_update_record_value >>>");

    check_useful_c_str!(type_, VcxErrorKind::InvalidOption);
    check_useful_c_str!(id, VcxErrorKind::InvalidOption);
    check_useful_c_str!(value, VcxErrorKind::InvalidOption);
    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_update_record_value(command_handle: {}, type_: {}, id: {}, value: {})",
           command_handle, secret!(&type_), secret!(&id), secret!(&value));

    spawn(move || {
        match wallet::update_record_value(&type_, &id, &value) {
            Ok(()) => {
                trace!("vcx_wallet_update_record_value(command_handle: {}, rc: {})",
                       command_handle, error::SUCCESS.message);

                cb(command_handle, error::SUCCESS.code_num);
            }
            Err(x) => {
                trace!("vcx_wallet_update_record_value(command_handle: {}, rc: {})",
                       command_handle, x);

                cb(command_handle, x.into());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Updates the value of a record tags already in the wallet.
/// Assumes there is an open wallet and that a type and id pair already exists.
/// #Params
///
/// command_handle: command handle to map callback to user context.
///
/// type_: type of record. (e.g. 'data', 'string', 'foobar', 'image')
///
/// id: the id ("key") of the record.
///
/// tags_json: New tags for the record with the associated id and type.
///
/// cb: Callback that any errors or a receipt of transfer
///
/// #Returns
/// Error code as a u32
///
#[no_mangle]
pub extern fn vcx_wallet_update_record_tags(command_handle: CommandHandle,
                                            type_: *const c_char,
                                            id: *const c_char,
                                            tags_json: *const c_char,
                                            cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32)>) -> u32 {
    info!("vcx_wallet_update_record_tags >>>");

    check_useful_c_str!(type_, VcxErrorKind::InvalidOption);
    check_useful_c_str!(id, VcxErrorKind::InvalidOption);
    check_useful_c_str!(tags_json, VcxErrorKind::InvalidOption);
    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_update_record_tags(command_handle: {}, type_: {}, id: {}, tags_json: {})",
           command_handle, secret!(&type_), secret!(&id), secret!(&tags_json));

    spawn(move || {
        match wallet::update_record_tags(&type_, &id, &tags_json) {
            Ok(()) => {
                trace!("vcx_wallet_update_record_tags(command_handle: {}, rc: {})",
                       command_handle, error::SUCCESS.message);

                cb(command_handle, error::SUCCESS.code_num);
            }
            Err(x) => {
                trace!("vcx_wallet_update_record_tags(command_handle: {}, rc: {})",
                       command_handle, x);

                cb(command_handle, x.into());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Adds tags to a record.
/// Assumes there is an open wallet and that a type and id pair already exists.
/// #Params
///
/// command_handle: command handle to map callback to user context.
///
/// type_: type of record. (e.g. 'data', 'string', 'foobar', 'image')
///
/// id: the id ("key") of the record.
///
/// tags_json: Tags for the record with the associated id and type.
///
/// cb: Callback that any errors or a receipt of transfer
///
/// #Returns
/// Error code as a u32
///
#[no_mangle]
pub extern fn vcx_wallet_add_record_tags(command_handle: CommandHandle,
                                         type_: *const c_char,
                                         id: *const c_char,
                                         tags_json: *const c_char,
                                         cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32)>) -> u32 {
    info!("vcx_wallet_add_record_tags >>>");

    check_useful_c_str!(type_, VcxErrorKind::InvalidOption);
    check_useful_c_str!(id, VcxErrorKind::InvalidOption);
    check_useful_c_str!(tags_json, VcxErrorKind::InvalidOption);
    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_add_record_tags(command_handle: {}, type_: {}, id: {}, tags_json: {})",
           command_handle, secret!(&type_), secret!(&id), secret!(&tags_json));

    spawn(move || {
        match wallet::add_record_tags(&type_, &id, &tags_json) {
            Ok(()) => {
                trace!("vcx_wallet_add_record_tags(command_handle: {}, rc: {})",
                       command_handle, error::SUCCESS.message);

                cb(command_handle, error::SUCCESS.code_num);
            }
            Err(x) => {
                trace!("vcx_wallet_add_record_tags(command_handle: {}, rc: {})",
                       command_handle, x);

                cb(command_handle, x.into());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Deletes tags from a record.
/// Assumes there is an open wallet and that a type and id pair already exists.
/// #Params
///
/// command_handle: command handle to map callback to user context.
///
/// type_: type of record. (e.g. 'data', 'string', 'foobar', 'image')
///
/// id: the id ("key") of the record.
///
/// tag_names_json: Tags to remove from the record with the associated id and type.
///
/// cb: Callback that any errors or a receipt of transfer
///
/// #Returns
/// Error code as a u32
///
#[no_mangle]
pub extern fn vcx_wallet_delete_record_tags(command_handle: CommandHandle,
                                            type_: *const c_char,
                                            id: *const c_char,
                                            tag_names_json: *const c_char,
                                            cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32)>) -> u32 {
    info!("vcx_wallet_delete_record_tags >>>");

    check_useful_c_str!(type_, VcxErrorKind::InvalidOption);
    check_useful_c_str!(id, VcxErrorKind::InvalidOption);
    check_useful_c_str!(tag_names_json, VcxErrorKind::InvalidOption);
    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_delete_record_tags(command_handle: {}, type_: {}, id: {}, tag_names_json: {})",
           command_handle, secret!(&type_), secret!(&id), secret!(&tag_names_json));

    spawn(move || {
        match wallet::delete_record_tags(&type_, &id, &tag_names_json) {
            Ok(()) => {
                trace!("vcx_wallet_delete_record_tags(command_handle: {}, rc: {})",
                       command_handle, error::SUCCESS.message);

                cb(command_handle, error::SUCCESS.code_num);
            }
            Err(x) => {
                trace!("vcx_wallet_delete_record_tags(command_handle: {}, rc: {})",
                       command_handle, x);

                cb(command_handle, x.into());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Deletes an existing record.
/// Assumes there is an open wallet and that a type and id pair already exists.
/// #Params
///
/// command_handle: command handle to map callback to user context.
///
/// type_: type of record. (e.g. 'data', 'string', 'foobar', 'image')
///
/// id: the id ("key") of the record.
///
/// cb: Callback that any errors or a receipt of transfer
///
/// #Returns
/// Error code as a u32
/// Error will be a libindy error code
///
#[no_mangle]
pub extern fn vcx_wallet_get_record(command_handle: CommandHandle,
                                    type_: *const c_char,
                                    id: *const c_char,
                                    options_json: *const c_char,
                                    cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32, record_json: *const c_char)>) -> u32 {
    info!("vcx_wallet_get_record >>>");

    check_useful_c_str!(type_, VcxErrorKind::InvalidOption);
    check_useful_c_str!(id, VcxErrorKind::InvalidOption);
    check_useful_c_str!(options_json, VcxErrorKind::InvalidOption);
    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_get_record(command_handle: {}, type_: {}, id: {}, options: {})",
           command_handle, secret!(&type_), secret!(&id), options_json);

    spawn(move || {
        match wallet::get_record(&type_, &id, &options_json) {
            Ok(x) => {
                trace!("vcx_wallet_get_record(command_handle: {}, rc: {}, record_json: {})",
                       command_handle, error::SUCCESS.message, x);

                let msg = CStringUtils::string_to_cstring(x);

                cb(command_handle, error::SUCCESS.code_num, msg.as_ptr());
            }
            Err(x) => {
                trace!("vcx_wallet_get_record(command_handle: {}, rc: {}, record_json: {})",
                       command_handle, x, "null");

                let msg = CStringUtils::string_to_cstring("".to_string());
                cb(command_handle, x.into(), msg.as_ptr());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Deletes an existing record.
/// Assumes there is an open wallet and that a type and id pair already exists.
/// #Params
///
/// command_handle: command handle to map callback to user context.
///
/// type_: type of record. (e.g. 'data', 'string', 'foobar', 'image')
///
/// id: the id ("key") of the record.
///
/// cb: Callback that any errors or a receipt of transfer
///
/// #Returns
/// Error code as a u32
/// Error will be a libindy error code
///
#[no_mangle]
pub extern fn vcx_wallet_delete_record(command_handle: CommandHandle,
                                       type_: *const c_char,
                                       id: *const c_char,
                                       cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32)>) -> u32 {
    info!("vcx_wallet_delete_record >>>");

    check_useful_c_str!(type_, VcxErrorKind::InvalidOption);
    check_useful_c_str!(id, VcxErrorKind::InvalidOption);
    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_delete_record(command_handle: {}, type_: {}, id: {})",
           command_handle, secret!(&type_), secret!(&id));

    spawn(move || {
        match wallet::delete_record(&type_, &id) {
            Ok(()) => {
                trace!("vcx_wallet_delete_record(command_handle: {}, rc: {})",
                       command_handle, error::SUCCESS.message);

                cb(command_handle, error::SUCCESS.code_num);
            }
            Err(x) => {
                trace!("vcx_wallet_delete_record(command_handle: {}, rc: {})",
                       command_handle, x);

                cb(command_handle, x.into());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}


/// Send tokens to a specific address
///
/// #Params
///
/// command_handle: command handle to map callback to user context.
///
/// payment_handle: for future use (currently uses any address in the wallet)
///
/// tokens: number of tokens to send
///
/// recipient: address of recipient
///
/// cb: Callback that any errors or a receipt of transfer
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_wallet_send_tokens(command_handle: CommandHandle,
                                     payment_handle: u32,
                                     tokens: *const c_char,
                                     recipient: *const c_char,
                                     cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32, receipt: *const c_char)>) -> u32 {
    info!("vcx_wallet_send_tokens >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    check_useful_c_str!(recipient, VcxErrorKind::InvalidOption);
    check_useful_c_str!(tokens, VcxErrorKind::InvalidOption);

    let tokens: u64 = match tokens.parse::<u64>() {
        Ok(x) => x,
        Err(e) => return VcxError::from_msg(VcxErrorKind::InvalidOption, format!("Cannot parse tokens: {}", e)).into(),
    };
    trace!("vcx_wallet_send_tokens(command_handle: {}, payment_handle: {}, tokens: {}, recipient: {})",
           command_handle, payment_handle, tokens, recipient);

    spawn(move || {
        match pay_a_payee(tokens, &recipient) {
            Ok((_payment, msg)) => {
                trace!("vcx_wallet_send_tokens_cb(command_handle: {}, rc: {}, receipt: {})",
                       command_handle, error::SUCCESS.message, msg);
                let msg = CStringUtils::string_to_cstring(msg);
                cb(command_handle, error::SUCCESS.code_num, msg.as_ptr());
            }
            Err(e) => {
                let msg = "Failed to send tokens".to_string();
                trace!("vcx_wallet_send_tokens_cb(command_handle: {}, rc: {}, reciept: {})", command_handle, e, msg);
                let msg = CStringUtils::string_to_cstring("".to_string());
                cb(command_handle, e.into(), msg.as_ptr());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Opens a storage search handle
///
/// #Params
///
/// command_handle: command handle to map callback to user context.
///
/// type_: type of record. (e.g. 'data', 'string', 'foobar', 'image')
///
/// query_json: MongoDB style query to wallet record tags:
///  {
///    "tagName": "tagValue",
///    $or: {
///      "tagName2": { $regex: 'pattern' },
///      "tagName3": { $gte: 123 },
///    },
///  }
/// options_json:
///  {
///    retrieveRecords: (optional, true by default) If false only "counts" will be calculated,
///    retrieveTotalCount: (optional, false by default) Calculate total count,
///    retrieveType: (optional, false by default) Retrieve record type,
///    retrieveValue: (optional, true by default) Retrieve record value,
///    retrieveTags: (optional, false by default) Retrieve record tags,
///  }
/// cb: Callback that any errors or a receipt of transfer
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_wallet_open_search(command_handle: CommandHandle,
                                     type_: *const c_char,
                                     query_json: *const c_char,
                                     options_json: *const c_char,
                                     cb: Option<extern fn(command_handle_: CommandHandle, err: u32,
                                                          search_handle: SearchHandle)>) -> u32 {
    info!("vcx_wallet_open_search >>>");

    check_useful_c_str!(type_, VcxErrorKind::InvalidOption);
    check_useful_c_str!(query_json, VcxErrorKind::InvalidOption);
    check_useful_c_str!(options_json, VcxErrorKind::InvalidOption);
    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_open_search(command_handle: {}, type_: {}, query_json: {}, options_json: {})",
           command_handle, secret!(&type_), secret!(&query_json), secret!(&options_json));

    spawn(move || {
        match wallet::open_search(&type_, &query_json, &options_json) {
            Ok(x) => {
                trace!("vcx_wallet_open_search(command_handle: {}, rc_: {}, search_handle: {})",
                       command_handle, error::SUCCESS.message, x);

                cb(command_handle, error::SUCCESS.code_num, x);
            }
            Err(x) => {
                trace!("vcx_wallet_get_record(command_handle: {}, rc: {}, record_json: {})",
                       command_handle, x, "null");

                cb(command_handle, x.into(), 0);
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Fetch next records for wallet search.
///
/// Not if there are no records this call returns WalletNoRecords error.
///
/// #Params
/// command_handle: command handle to map callback to user context.
/// wallet_search_handle: wallet search handle (created by vcx_wallet_open_search)
/// count: Count of records to fetch
///
/// #Returns
/// wallet records json:
/// {
///   totalCount: <int>, // present only if retrieveTotalCount set to true
///   records: [{ // present only if retrieveRecords set to true
///       id: "Some id",
///       type: "Some type", // present only if retrieveType set to true
///       value: "Some value", // present only if retrieveValue set to true
///       tags: <tags json>, // present only if retrieveTags set to true
///   }],
/// }
#[no_mangle]
pub extern fn vcx_wallet_search_next_records(command_handle: CommandHandle,
                                             wallet_search_handle: SearchHandle,
                                             count: usize,
                                             cb: Option<extern fn(command_handle_: CommandHandle, err: u32,
                                                                  records_json: *const c_char)>) -> u32 {
    info!("vcx_wallet_search_next_records >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_search_next_records(command_handle: {}, wallet_search_handle: {})",
           command_handle, wallet_search_handle);

    spawn(move || {
        match wallet::fetch_next_records(wallet_search_handle, count) {
            Ok(x) => {
                trace!("vcx_wallet_search_next_records(command_handle: {}, rc: {}, record_json: {})",
                       command_handle, error::SUCCESS.message, x);

                let msg = CStringUtils::string_to_cstring(x);

                cb(command_handle, error::SUCCESS.code_num, msg.as_ptr());
            }
            Err(x) => {
                trace!("vcx_wallet_get_record(command_handle: {}, rc: {}, record_json: {})",
                       command_handle, x, "null");

                let msg = CStringUtils::string_to_cstring("".to_string());
                cb(command_handle, x.into(), msg.as_ptr());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Close a search
///
/// #Params
///
/// command_handle: command handle to map callback to user context.
///
/// search_handle: wallet search handle
///
/// cb: Callback that any errors or a receipt of transfer
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_wallet_close_search(command_handle: CommandHandle,
                                      search_handle: SearchHandle,
                                      cb: Option<extern fn(xcommand_handle: CommandHandle, err: u32)>) -> u32 {
    info!("vcx_wallet_close_search >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_close_search(command_handle: {}, search_handle: {})",
           command_handle, search_handle);

    spawn(move || {
        trace!("vcx_wallet_close_search(command_handle: {}, rc: {})",
               command_handle, error::SUCCESS.message);
        match wallet::close_search(search_handle) {
            Ok(()) => {
                trace!("vcx_wallet_close_search(command_handle: {}, rc: {})", command_handle, error::SUCCESS.message);
                cb(command_handle, error::SUCCESS.code_num);
            }
            Err(e) => {
                trace!("vcx_wallet_close_search(command_handle: {}, rc: {})", command_handle, e);
                cb(command_handle, e.into());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Exports opened wallet
///
/// Note this endpoint is EXPERIMENTAL. Function signature and behaviour may change
/// in the future releases.
///
/// #Params:
/// command_handle: Handle for User's Reference only.
/// path: Path to export wallet to User's File System.
/// backup_key: String representing the User's Key for securing (encrypting) the exported Wallet.
/// cb: Callback that provides the success/failure of the api call.
/// #Returns
/// Error code - success indicates that the api call was successfully created and execution
/// is scheduled to begin in a separate thread.
#[no_mangle]
pub extern fn vcx_wallet_export(command_handle: CommandHandle,
                                path: *const c_char,
                                backup_key: *const c_char,
                                cb: Option<extern fn(xcommand_handle: CommandHandle,
                                                     err: u32)>) -> u32 {
    info!("vcx_wallet_export >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    check_useful_c_str!(path,  VcxErrorKind::InvalidOption);
    check_useful_c_str!(backup_key, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_export(command_handle: {}, path: {}, backup_key: ****)",
           command_handle, path);


    spawn(move || {
        trace!("vcx_wallet_export(command_handle: {}, path: {}, backup_key: ****)", command_handle, path);
        match export_main_wallet(&path, &backup_key) {
            Ok(()) => {
                let return_code = error::SUCCESS.code_num;
                trace!("vcx_wallet_export(command_handle: {}, rc: {})", command_handle, return_code);
                cb(command_handle, return_code);
            }
            Err(e) => {
                warn!("vcx_wallet_export(command_handle: {}, rc: {})", command_handle, e);
                cb(command_handle, e.into());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Creates a new secure wallet and then imports its content
/// according to fields provided in import_config
/// Cannot be used if wallet is already opened (Especially if vcx_init has already been used).
///
/// Note this only works for default storage type (file), as currently this function does not let
/// you pass down information about wallet storage_type, storage_config, storage_credentials.
///
/// Note this endpoint is EXPERIMENTAL. Function signature and behaviour may change
/// in the future releases.
///
/// config: "{"wallet_name":"","wallet_key":"","exported_wallet_path":"","backup_key":"","key_derivation":""}"
/// exported_wallet_path: Path of the file that contains exported wallet content
/// backup_key: Key used when creating the backup of the wallet (For encryption/decrption)
/// Optional<key_derivation>: method of key derivation used by libindy. By default, libvcx uses ARGON2I_INT
/// cb: Callback that provides the success/failure of the api call.
/// #Returns
/// Error code - success indicates that the api call was successfully created and execution
/// is scheduled to begin in a separate thread.
#[no_mangle]
pub extern fn vcx_wallet_import(command_handle: CommandHandle,
                                config: *const c_char,
                                cb: Option<extern fn(xcommand_handle: CommandHandle,
                                                     err: u32)>) -> u32 {
    info!("vcx_wallet_import >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    check_useful_c_str!(config,  VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_import(command_handle: {}, config: ****)",
           command_handle);

    thread::spawn(move || {
        trace!("vcx_wallet_import(command_handle: {}, config: ****)", command_handle);
        match import(&config) {
            Ok(()) => {
                trace!("vcx_wallet_import(command_handle: {}, rc: {})", command_handle, error::SUCCESS.message);
                cb(command_handle, error::SUCCESS.code_num);
            }
            Err(e) => {
                warn!("vcx_wallet_import(command_handle: {}, rc: {})", command_handle, e);
                cb(command_handle, e.into());
            }
        };
    });

    error::SUCCESS.code_num
}

// Functionality in Libindy for validating an address in NOT there yet
/// Validates a Payment address
///
/// #Params
///
/// command_handle: command handle to map callback to user context.
///
/// payment_address: value to be validated as a payment address
///
/// cb: Callback that any errors
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_wallet_validate_payment_address(command_handle: i32,
                                                  payment_address: *const c_char,
                                                  cb: Option<extern fn(command_handle_: i32, err: u32)>) -> u32 {
    info!("vcx_wallet_validate_payment_address >>>");

    check_useful_c_str!(payment_address,  VcxErrorKind::InvalidOption);
    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);

    trace!("vcx_wallet_validate_payment_address(command_handle: {}, payment_address: {})",
           command_handle, payment_address);

    spawn(move || {
        cb(command_handle, error::SUCCESS.code_num);
        Ok(())
    });

    error::SUCCESS.code_num
}

/// Set the wallet handle before calling vcx_init_minimal
///
/// #params
///
/// handle: wallet handle that libvcx should use
///
/// #Returns
/// Error code as u32
#[no_mangle]
pub extern fn vcx_wallet_set_handle(handle: WalletHandle) -> WalletHandle {
    wallet::set_wallet_handle(handle)
}

#[cfg(test)]
pub mod tests {
    extern crate serde_json;

    use std::ffi::CString;
    use std::ptr;

    use crate::{libindy, settings};
    use crate::api::return_types_u32;
    #[cfg(feature = "pool_tests")]
    use crate::libindy::utils::payments::build_test_address;
    use crate::libindy::utils::wallet::{close_main_wallet, create_and_open_as_main_wallet, delete_wallet};
    use crate::utils::devsetup::*;
    use crate::utils::timeout::TimeoutUtils;

    use super::*;

    #[test]
    #[cfg(feature = "general_test")]
    fn test_get_token_info() {
        let _setup = SetupMocks::init();

        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(vcx_wallet_get_token_info(cb.command_handle,
                                             0,
                                             Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        cb.receive(TimeoutUtils::some_medium()).unwrap();
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_send_tokens() {
        let _setup = SetupMocks::init();

        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(vcx_wallet_send_tokens(cb.command_handle,
                                          0,
                                          CString::new("1").unwrap().into_raw(),
                                          CString::new("address").unwrap().into_raw(),
                                          Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        cb.receive(TimeoutUtils::some_medium()).unwrap();
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_create_address() {
        let _setup = SetupMocks::init();

        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(vcx_wallet_create_payment_address(cb.command_handle,
                                                     ptr::null_mut(),
                                                     Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        cb.receive(TimeoutUtils::some_medium()).unwrap();
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_sign_with_address_api() {
        let _setup = SetupMocks::init();

        let cb = return_types_u32::Return_U32_BIN::new().unwrap();
        let msg = "message";
        let msg_len = msg.len();
        let msg_raw = CString::new(msg).unwrap();
        assert_eq!(vcx_wallet_sign_with_address(cb.command_handle,
                                                CString::new("address").unwrap().into_raw(),
                                                msg_raw.as_ptr() as *const u8,
                                                msg_len as u32,
                                                Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        let res = cb.receive(TimeoutUtils::some_medium()).unwrap();
        assert_eq!(msg.as_bytes(), res.as_slice());
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_verify_with_address_api() {
        let _setup = SetupMocks::init();

        let cb = return_types_u32::Return_U32_BOOL::new().unwrap();
        let msg = "message";
        let msg_len = msg.len();
        let msg_raw = CString::new(msg).unwrap();
        let sig = "signature";
        let sig_len = sig.len();
        let sig_raw = CString::new(sig).unwrap();
        assert_eq!(vcx_wallet_verify_with_address(cb.command_handle,
                                                  CString::new("address").unwrap().into_raw(),
                                                  msg_raw.as_ptr() as *const u8,
                                                  msg_len as u32,
                                                  sig_raw.as_ptr() as *const u8,
                                                  sig_len as u32,
                                                  Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        let res = cb.receive(TimeoutUtils::some_medium()).unwrap();
        assert!(res);
    }

    #[cfg(feature = "pool_tests")]
    #[test]
    fn test_sign_verify_with_address() {
        let _setup = SetupLibraryWalletPoolZeroFees::init();

        let cb_sign = return_types_u32::Return_U32_BIN::new().unwrap();
        let cb_verify = return_types_u32::Return_U32_BOOL::new().unwrap();
        let cb_addr = return_types_u32::Return_U32_STR::new().unwrap();

        let msg = "message";
        let msg_len = msg.len();
        let msg_raw = CString::new(msg).unwrap();

        vcx_wallet_create_payment_address(cb_addr.command_handle,
                                          ptr::null_mut(),
                                          Some(cb_addr.get_callback()));
        let addr = cb_addr.receive(TimeoutUtils::some_medium()).unwrap().unwrap();
        let addr_raw = CString::new(addr.clone()).unwrap();

        let res_sign = vcx_wallet_sign_with_address(cb_sign.command_handle,
                                                    addr_raw.into_raw(),
                                                    msg_raw.as_ptr() as *const u8,
                                                    msg_len as u32,
                                                    Some(cb_sign.get_callback()));
        assert_eq!(res_sign, error::SUCCESS.code_num);

        let addr_raw = CString::new(addr).unwrap();
        let sig = cb_sign.receive(TimeoutUtils::some_medium()).unwrap();

        let res_verify = vcx_wallet_verify_with_address(cb_verify.command_handle,
                                                        addr_raw.into_raw(),
                                                        msg_raw.as_ptr() as *const u8,
                                                        msg_len as u32,
                                                        sig.as_ptr(),
                                                        sig.len() as u32,
                                                        Some(cb_verify.get_callback()));
        assert_eq!(res_verify, error::SUCCESS.code_num);
        let valid = cb_verify.receive(TimeoutUtils::some_medium()).unwrap();
        assert!(valid);
    }

    #[cfg(feature = "pool_tests")]
    #[test]
    fn test_send_payment() {
        let _setup = SetupLibraryWalletPoolZeroFees::init();

        let recipient = CStringUtils::string_to_cstring(build_test_address("2ZrAm5Jc3sP4NAXMQbaWzDxEa12xxJW3VgWjbbPtMPQCoznJyS"));
        debug!("sending payment to {:?}", recipient);
        let balance = libindy::utils::payments::get_wallet_token_info().unwrap().get_balance();
        let tokens = 5;
        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(vcx_wallet_send_tokens(cb.command_handle,
                                          0,
                                          CString::new(format!("{}", tokens)).unwrap().into_raw(),
                                          recipient.as_ptr(),
                                          Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        cb.receive(TimeoutUtils::some_medium()).unwrap();
        let new_balance = libindy::utils::payments::get_wallet_token_info().unwrap().get_balance();
        assert_eq!(balance - tokens, new_balance);
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_add_record() {
        let _setup = SetupLibraryWallet::init();

        let xtype = CStringUtils::string_to_cstring("record_type".to_string());
        let id = CStringUtils::string_to_cstring("123".to_string());
        let value = CStringUtils::string_to_cstring("Record Value".to_string());
        let tags = CStringUtils::string_to_cstring("{}".to_string());

        // Valid add
        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_wallet_add_record(cb.command_handle,
                                         xtype.as_ptr(),
                                         id.as_ptr(),
                                         value.as_ptr(),
                                         tags.as_ptr(),
                                         Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        cb.receive(TimeoutUtils::some_medium()).unwrap();

        // Failure because of duplicate
        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_wallet_add_record(cb.command_handle,
                                         xtype.as_ptr(),
                                         id.as_ptr(),
                                         value.as_ptr(),
                                         tags.as_ptr(),
                                         Some(cb.get_callback())),
                   error::SUCCESS.code_num);

        assert_eq!(cb.receive(TimeoutUtils::some_medium()).err(), Some(error::DUPLICATE_WALLET_RECORD.code_num));
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_add_record_with_tag() {
        let _setup = SetupLibraryWallet::init();

        let xtype = CStringUtils::string_to_cstring("record_type".to_string());
        let id = CStringUtils::string_to_cstring("123".to_string());
        let value = CStringUtils::string_to_cstring("Record Value".to_string());
        let tags = CStringUtils::string_to_cstring(r#"{"tagName1":"tag1","tagName2":"tag2"}"#.to_string());

        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_wallet_add_record(cb.command_handle,
                                         xtype.as_ptr(),
                                         id.as_ptr(),
                                         value.as_ptr(),
                                         tags.as_ptr(),
                                         Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        cb.receive(TimeoutUtils::some_medium()).unwrap();
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_get_record_fails_with_no_value() {
        let _setup = SetupLibraryWallet::init();

        let xtype = CStringUtils::string_to_cstring("record_type".to_string());
        let id = CStringUtils::string_to_cstring("123".to_string());
        let options = json!({
            "retrieveType": true,
            "retrieveValue": true,
            "retrieveTags": false
        }).to_string();
        let options = CStringUtils::string_to_cstring(options);

        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(vcx_wallet_get_record(cb.command_handle,
                                         xtype.as_ptr(),
                                         id.as_ptr(),
                                         options.as_ptr(),
                                         Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        assert_eq!(cb.receive(TimeoutUtils::some_medium()).err(), Some(error::WALLET_RECORD_NOT_FOUND.code_num));
    }

    pub fn _test_add_and_get_wallet_record() {
        let xtype = CStringUtils::string_to_cstring("record_type".to_string());
        let id = CStringUtils::string_to_cstring("123".to_string());
        let value = CStringUtils::string_to_cstring("Record Value".to_string());
        let tags = CStringUtils::string_to_cstring("{}".to_string());
        let options = json!({
            "retrieveType": true,
            "retrieveValue": true,
            "retrieveTags": false
        }).to_string();
        let options = CStringUtils::string_to_cstring(options);

        // Valid add
        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_wallet_add_record(cb.command_handle,
                                         xtype.as_ptr(),
                                         id.as_ptr(),
                                         value.as_ptr(),
                                         tags.as_ptr(),
                                         Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        cb.receive(TimeoutUtils::some_custom(1)).unwrap();

        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(vcx_wallet_get_record(cb.command_handle,
                                         xtype.as_ptr(),
                                         id.as_ptr(),
                                         options.as_ptr(),
                                         Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        let record_value = cb.receive(TimeoutUtils::some_custom(1)).unwrap().unwrap();
        assert!(record_value.contains("Record Value"));
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_get_record_value_success() {
        let _setup = SetupLibraryWallet::init();
        _test_add_and_get_wallet_record();
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_delete_record() {
        let _setup = SetupLibraryWallet::init();

        let xtype = CStringUtils::string_to_cstring("record_type".to_string());
        let id = CStringUtils::string_to_cstring("123".to_string());
        let value = CStringUtils::string_to_cstring("Record Value".to_string());
        let tags = CStringUtils::string_to_cstring("{}".to_string());

        // Add record
        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_wallet_add_record(cb.command_handle, xtype.as_ptr(),
                                         id.as_ptr(),
                                         value.as_ptr(),
                                         tags.as_ptr(),
                                         Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        cb.receive(TimeoutUtils::some_medium()).unwrap();

        // Successful deletion
        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_wallet_delete_record(cb.command_handle,
                                            xtype.as_ptr(),
                                            id.as_ptr(),
                                            Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        cb.receive(TimeoutUtils::some_medium()).unwrap();

        // Fails with no record
        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_wallet_delete_record(cb.command_handle,
                                            xtype.as_ptr(),
                                            id.as_ptr(),
                                            Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        assert_eq!(cb.receive(TimeoutUtils::some_medium()).err(),
                   Some(error::WALLET_RECORD_NOT_FOUND.code_num));
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_update_record_value() {
        let _setup = SetupLibraryWallet::init();

        let xtype = CStringUtils::string_to_cstring("record_type".to_string());
        let id = CStringUtils::string_to_cstring("123".to_string());
        let value = CStringUtils::string_to_cstring("Record Value".to_string());
        let tags = CStringUtils::string_to_cstring("{}".to_string());
        let options = json!({
            "retrieveType": true,
            "retrieveValue": true,
            "retrieveTags": false
        }).to_string();
        let options = CStringUtils::string_to_cstring(options);

        // Assert no record to update
        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_wallet_update_record_value(cb.command_handle,
                                                  xtype.as_ptr(),
                                                  id.as_ptr(),
                                                  options.as_ptr(),
                                                  Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        assert_eq!(cb.receive(TimeoutUtils::some_medium()).err(),
                   Some(error::WALLET_RECORD_NOT_FOUND.code_num));

        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_wallet_add_record(cb.command_handle, xtype.as_ptr(),
                                         id.as_ptr(),
                                         value.as_ptr(),
                                         tags.as_ptr(),
                                         Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        cb.receive(TimeoutUtils::some_medium()).unwrap();

        // Assert update works
        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_wallet_update_record_value(cb.command_handle,
                                                  xtype.as_ptr(),
                                                  id.as_ptr(),
                                                  options.as_ptr(),
                                                  Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        cb.receive(TimeoutUtils::some_medium()).unwrap();
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_wallet_export_import() {
        let _setup = SetupDefaults::init();

        let wallet_name = "test_wallet_import_export";

        let export_file = TempFile::prepare_path(wallet_name);

        create_and_open_as_main_wallet(wallet_name, settings::DEFAULT_WALLET_KEY, settings::WALLET_KDF_RAW, None, None, None).unwrap();

        let backup_key = settings::get_config_value(settings::CONFIG_WALLET_BACKUP_KEY).unwrap();
        let wallet_key = settings::get_config_value(settings::CONFIG_WALLET_KEY).unwrap();

        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_wallet_export(cb.command_handle,
                                     CString::new(export_file.path.clone()).unwrap().as_ptr(),
                                     CString::new(backup_key.clone()).unwrap().as_ptr(),
                                     Some(cb.get_callback())), error::SUCCESS.code_num);
        cb.receive(TimeoutUtils::some_long()).unwrap();

        close_main_wallet().unwrap();
        delete_wallet(&wallet_name, settings::DEFAULT_WALLET_KEY, settings::WALLET_KDF_RAW, None, None, None).unwrap();

        let import_config = json!({
            settings::CONFIG_WALLET_NAME: wallet_name,
            settings::CONFIG_WALLET_KEY: wallet_key,
            settings::CONFIG_EXPORTED_WALLET_PATH: export_file.path,
            settings::CONFIG_WALLET_BACKUP_KEY: backup_key,
            settings::CONFIG_WALLET_KEY_DERIVATION: settings::WALLET_KDF_RAW,
        }).to_string();

        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_wallet_import(cb.command_handle,
                                     CString::new(import_config).unwrap().as_ptr(),
                                     Some(cb.get_callback())), error::SUCCESS.code_num);
        cb.receive(TimeoutUtils::some_long()).unwrap();

        delete_wallet(&wallet_name, settings::DEFAULT_WALLET_KEY, settings::WALLET_KDF_RAW, None, None, None).unwrap();
    }
}
