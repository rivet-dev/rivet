//! Opaque transaction identities. Tags distinguish generic and staged-write transactions.
use rivet_keys::keys::KeyAddress;
rivet_keys::volume_kv_key! {
	TxnKey(kind: u64, owner: u64, counter: u64) => [{kind}, {owner}, {counter}];
}
pub(crate) fn client_txn(epoch: u64, counter: u64) -> Vec<u8> {
	TxnKey::new(1, epoch, counter).pack().to_vec()
}
pub(crate) fn staged_write_txn(ino: u64, base: u64) -> Vec<u8> {
	TxnKey::new(2, ino, base).pack().to_vec()
}
