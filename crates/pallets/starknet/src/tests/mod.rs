use blockifier::abi::abi_utils::get_erc20_balance_var_addresses;
use blockifier::state::state_api::State;
use mp_felt::Felt252Wrapper;
use mp_transactions::compute_hash::ComputeTransactionHash;
use mp_transactions::{DeclareTransaction, DeclareTransactionV1, DeployAccountTransaction, InvokeTransactionV1};
use starknet_api::api_core::{ContractAddress, Nonce};
use starknet_api::hash::StarkFelt;

use self::mock::default_mock::{MockRuntime, Starknet};
use self::mock::{get_account_address, AccountType};
use crate::blockifier_state_adapter::BlockifierStateAdapter;
use crate::tests::mock::account_helper;
use crate::tests::utils::sign_message_hash;
use crate::{Config, Nonces};

mod account_helper;
mod call_contract;
mod declare_tx;
mod deploy_account_tx;
mod erc20;
mod events;
mod fees_disabled;
mod invoke_tx;
mod l1_handler_validation;
mod l1_message;
mod no_nonce_validation;
mod query_tx;
mod send_message;
mod sequencer_address;

mod block;
mod constants;
mod mock;
mod utils;

// ref: https://github.com/tdelabro/blockifier/blob/no_std-support/crates/blockifier/feature_contracts/account_without_validations.cairo
pub fn get_invoke_dummy(nonce: Felt252Wrapper) -> InvokeTransactionV1 {
    let signature = vec![
        Felt252Wrapper::from_hex_be("0x00f513fe663ffefb9ad30058bb2d2f7477022b149a0c02fb63072468d3406168").unwrap(),
        Felt252Wrapper::from_hex_be("0x02e29e92544d31c03e89ecb2005941c88c28b4803a3647a7834afda12c77f096").unwrap(),
    ];
    let sender_address = Felt252Wrapper::from_hex_be(constants::BLOCKIFIER_ACCOUNT_ADDRESS).unwrap();
    let calldata = vec![
        Felt252Wrapper::from_hex_be("0x024d1e355f6b9d27a5a420c8f4b50cea9154a8e34ad30fc39d7c98d3c177d0d7").unwrap(), /* contract_address */
        Felt252Wrapper::from_hex_be("0x00e7def693d16806ca2a2f398d8de5951344663ba77f340ed7a958da731872fc").unwrap(), /* selector for the `with_arg` external */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(), /* calldata_len */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000019").unwrap(), /* calldata[0] */
    ];

    InvokeTransactionV1 { max_fee: u64::MAX as u128, signature, nonce, sender_address, calldata, offset_version: false }
}

// ref: https://github.com/argentlabs/argent-contracts-starknet/blob/develop/contracts/account/ArgentAccount.cairo
fn get_invoke_argent_dummy() -> InvokeTransactionV1 {
    let sender_address =
        Felt252Wrapper::from_hex_be("0x02e63de215f650e9d7e2313c6e9ed26b4f920606fb08576b1663c21a7c4a28c5").unwrap();
    let nonce = Felt252Wrapper::ZERO;
    let calldata = vec![
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(), /* call_array_len */
        Felt252Wrapper::from_hex_be("0x024d1e355f6b9d27a5a420c8f4b50cea9154a8e34ad30fc39d7c98d3c177d0d7").unwrap(), /* to */
        Felt252Wrapper::from_hex_be("0x00e7def693d16806ca2a2f398d8de5951344663ba77f340ed7a958da731872fc").unwrap(), /* selector */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap(), /* data_offset */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(), /* data_len */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(), /* calldata_len */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000019").unwrap(), /* calldata[0] */
    ];

    InvokeTransactionV1 {
        max_fee: u64::MAX as u128,
        signature: vec![],
        nonce,
        sender_address,
        calldata,
        offset_version: false,
    }
}

// ref: https://github.com/myBraavos/braavos-account-cairo/blob/develop/src/account/Account.cairo
fn get_invoke_braavos_dummy() -> InvokeTransactionV1 {
    let signature = vec![
        Felt252Wrapper::from_hex_be("0x00f513fe663ffefb9ad30058bb2d2f7477022b149a0c02fb63072468d3406168").unwrap(),
        Felt252Wrapper::from_hex_be("0x02e29e92544d31c03e89ecb2005941c88c28b4803a3647a7834afda12c77f096").unwrap(),
    ];
    let sender_address =
        Felt252Wrapper::from_hex_be("0x05ef3fba22df259bf84890945352df711bcc9a4e3b6858cb93e9c90d053cf122").unwrap();
    let nonce = Felt252Wrapper::ZERO;
    let calldata = vec![
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(), /* call_array_len */
        Felt252Wrapper::from_hex_be("0x024d1e355f6b9d27a5a420c8f4b50cea9154a8e34ad30fc39d7c98d3c177d0d7").unwrap(), /* to */
        Felt252Wrapper::from_hex_be("0x00e7def693d16806ca2a2f398d8de5951344663ba77f340ed7a958da731872fc").unwrap(), /* selector */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap(), /* data_offset */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(), /* data_len */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(), /* calldata_len */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000019").unwrap(), /* calldata[0] */
    ];

    InvokeTransactionV1 { max_fee: u64::MAX as u128, signature, nonce, sender_address, calldata, offset_version: false }
}

// ref: https://github.com/OpenZeppelin/cairo-contracts/blob/main/src/openzeppelin/token/erc20/IERC20.cairo
fn get_invoke_emit_event_dummy() -> InvokeTransactionV1 {
    let signature = vec![
        Felt252Wrapper::from_hex_be("0x00f513fe663ffefb9ad30058bb2d2f7477022b149a0c02fb63072468d3406168").unwrap(),
        Felt252Wrapper::from_hex_be("0x02e29e92544d31c03e89ecb2005941c88c28b4803a3647a7834afda12c77f096").unwrap(),
    ];
    let sender_address =
        Felt252Wrapper::from_hex_be("0x01a3339ec92ac1061e3e0f8e704106286c642eaf302e94a582e5f95ef5e6b4d0").unwrap();
    let nonce = Felt252Wrapper::ZERO;
    let calldata = vec![
        Felt252Wrapper::from_hex_be("0x024d1e355f6b9d27a5a420c8f4b50cea9154a8e34ad30fc39d7c98d3c177d0d7").unwrap(), /* to */
        Felt252Wrapper::from_hex_be("0x00966af5d72d3975f70858b044c77785d3710638bbcebbd33cc7001a91025588").unwrap(), /* selector */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap(), /* amount */
    ];

    InvokeTransactionV1 { max_fee: u64::MAX as u128, signature, nonce, sender_address, calldata, offset_version: false }
}

// ref: https://github.com/tdelabro/blockifier/blob/no_std-support/crates/blockifier/feature_contracts/account_without_validations.cairo
fn get_invoke_nonce_dummy() -> InvokeTransactionV1 {
    let signature = vec![
        Felt252Wrapper::from_hex_be("0x00f513fe663ffefb9ad30058bb2d2f7477022b149a0c02fb63072468d3406168").unwrap(),
        Felt252Wrapper::from_hex_be("0x02e29e92544d31c03e89ecb2005941c88c28b4803a3647a7834afda12c77f096").unwrap(),
    ];
    let sender_address = Felt252Wrapper::from_hex_be(constants::BLOCKIFIER_ACCOUNT_ADDRESS).unwrap();
    let nonce = Felt252Wrapper::ONE;
    let calldata = vec![
        Felt252Wrapper::from_hex_be("0x024d1e355f6b9d27a5a420c8f4b50cea9154a8e34ad30fc39d7c98d3c177d0d7").unwrap(), /* contract_address */
        Felt252Wrapper::from_hex_be("0x00e7def693d16806ca2a2f398d8de5951344663ba77f340ed7a958da731872fc").unwrap(), /* selector */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(), /* calldata_len */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000019").unwrap(), /* calldata[0] */
    ];

    InvokeTransactionV1 { max_fee: u64::MAX as u128, signature, nonce, sender_address, calldata, offset_version: false }
}

// ref: https://github.com/keep-starknet-strange/madara/blob/main/cairo-contracts/src/accounts/NoValidateAccount.cairo
fn get_storage_read_write_dummy() -> InvokeTransactionV1 {
    let signature = vec![];
    let sender_address = Felt252Wrapper::from_hex_be(constants::BLOCKIFIER_ACCOUNT_ADDRESS).unwrap();
    let nonce = Felt252Wrapper::ZERO;
    let calldata = vec![
        Felt252Wrapper::from_hex_be("0x024d1e355f6b9d27a5a420c8f4b50cea9154a8e34ad30fc39d7c98d3c177d0d7").unwrap(), /* contract_address */
        Felt252Wrapper::from_hex_be("0x03b097c62d3e4b85742aadd0dfb823f96134b886ec13bda57b68faf86f294d97").unwrap(), /* selector */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000002").unwrap(), /* calldata_len */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000019").unwrap(), /* calldata[0] */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(), /* calldata[1] */
    ];

    InvokeTransactionV1 { max_fee: u64::MAX as u128, signature, nonce, sender_address, calldata, offset_version: false }
}

// ref: https://github.com/OpenZeppelin/cairo-contracts/blob/main/src/openzeppelin/account/IAccount.cairo
fn get_invoke_openzeppelin_dummy() -> InvokeTransactionV1 {
    let signature = vec![
        Felt252Wrapper::from_hex_be("0x028ef1ae6c37314bf9df65663db1cf68f95d67c4b4cf7f6590654933a84912b0").unwrap(),
        Felt252Wrapper::from_hex_be("0x0625aae99c58b18e5161c719fef0f99579c6468ca6c1c866f9b2b968a5447e4").unwrap(),
    ];
    let sender_address =
        Felt252Wrapper::from_hex_be("0x06e2616a2dceff4355997369246c25a78e95093df7a49e5ca6a06ce1544ffd50").unwrap();
    let nonce = Felt252Wrapper::ZERO;
    let calldata = vec![
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(), /* call_array_len */
        Felt252Wrapper::from_hex_be("0x024d1e355f6b9d27a5a420c8f4b50cea9154a8e34ad30fc39d7c98d3c177d0d7").unwrap(), /* to */
        Felt252Wrapper::from_hex_be("0x00e7def693d16806ca2a2f398d8de5951344663ba77f340ed7a958da731872fc").unwrap(), /* selector */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap(), /* data offset */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(), /* data length */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(), /* calldata_len */
        Felt252Wrapper::from_hex_be("0x0000000000000000000000000000000000000000000000000000000000000019").unwrap(), /* calldata[0] */
    ];

    InvokeTransactionV1 { max_fee: u64::MAX as u128, signature, nonce, sender_address, calldata, offset_version: false }
}

/// Returns a dummy declare transaction for the given account type.
/// The declared class hash is a ERC20 contract, class hash calculated
/// with starkli.
pub fn get_declare_dummy(
    chain_id: Felt252Wrapper,
    nonce: Felt252Wrapper,
    account_type: AccountType,
) -> DeclareTransaction {
    let account_addr = get_account_address(None, account_type);

    let erc20_class_hash =
        Felt252Wrapper::from_hex_be("0x372ee6669dc86563007245ed7343d5180b96221ce28f44408cff2898038dbd4").unwrap();

    let mut tx = DeclareTransactionV1 {
        max_fee: u64::MAX as u128,
        signature: vec![],
        nonce,
        class_hash: erc20_class_hash,
        sender_address: account_addr.into(),
        offset_version: false,
    };

    let tx_hash = tx.compute_hash::<<MockRuntime as Config>::SystemHash>(chain_id, false);

    let signature = sign_message_hash(tx_hash);
    tx.signature = signature;

    tx.into()
}

/// Returns a dummy deploy account transaction for the given salt and account type
pub fn get_deploy_account_dummy(
    nonce: Felt252Wrapper,
    salt: Felt252Wrapper,
    account_type: AccountType,
) -> DeployAccountTransaction {
    let (account_class_hash, calldata) = account_helper(account_type);

    DeployAccountTransaction {
        max_fee: u64::MAX as u128,
        signature: vec![],
        nonce,
        contract_address_salt: salt,
        constructor_calldata: calldata.0.iter().map(|e| Felt252Wrapper::from(*e)).collect(),
        class_hash: account_class_hash.into(),
        offset_version: false,
    }
}

/// Sets the balance of the given address to infinite.
pub fn set_infinite_tokens<T: Config>(contract_address: &ContractAddress) {
    let fee_token_address = Starknet::fee_token_address();
    let (low_key, high_key) = get_erc20_balance_var_addresses(contract_address).unwrap();
    let mut state_adapter = BlockifierStateAdapter::<T>::default();

    state_adapter.set_storage_at(fee_token_address, low_key, StarkFelt::from(u64::MAX as u128));
    state_adapter.set_storage_at(fee_token_address, high_key, StarkFelt::from(u64::MAX as u128));
}

/// Sets nonce for the given address.
pub fn set_nonce<T: Config>(address: &ContractAddress, nonce: &Nonce) {
    Nonces::<T>::insert(address, nonce)
}
