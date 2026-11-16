//! Opaque transaction identities. Tags distinguish generic and staged-write transactions.
use rivet_keys::keys::KeyAddress;
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [kind, owner, counter])]
struct TxnKey {
	pub kind: u64,
	pub owner: u64,
	pub counter: u64,
}
pub(crate) fn client_txn(epoch: u64, counter: u64) -> Vec<u8> {
	TxnKey::new(1, epoch, counter).pack().to_vec()
}
pub(crate) fn staged_write_txn(ino: u64, base: u64) -> Vec<u8> {
	TxnKey::new(2, ino, base).pack().to_vec()
}
