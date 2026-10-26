#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use std::sync::Arc;

use object_store::{memory::InMemory, ObjectStore};
use rand::{rngs::StdRng, Rng, SeedableRng};

#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn test_concurrent_bank_transfers() {
    let num_accounts = 100;
    let initial_balance = 10_000u64;
    let expected_total = initial_balance * num_accounts as u64;

    fn account_key(id: usize) -> Vec<u8> {
        format!("acct_{:04}", id).into_bytes()
    }

    async fn get_balance(txn: &slatedb::DbTransaction, key: &[u8]) -> u64 {
        u64::from_le_bytes(
            txn.get(key)
                .await
                .unwrap()
                .unwrap()
                .as_ref()
                .try_into()
                .unwrap(),
        )
    }

    let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    let db = Arc::new(
        slatedb::Db::open("test_concurrency_issue", object_store)
            .await
            .unwrap(),
    );

    for i in 0..num_accounts {
        db.put(&account_key(i), &initial_balance.to_le_bytes())
            .await
            .unwrap();
    }
    db.flush().await.unwrap();

    let handles: Vec<_> = (0..100)
        .map(|_| {
            let db = db.clone();
            tokio::spawn(async move {
                let mut rng = StdRng::from_os_rng();
                for _ in 0..100 {
                    let from = rng.random_range(0..num_accounts);
                    let mut to = rng.random_range(0..num_accounts);
                    while to == from {
                        to = rng.random_range(0..num_accounts);
                    }
                    loop {
                        let txn = db.begin(slatedb::IsolationLevel::Snapshot).await.unwrap();
                        let from_bal = get_balance(&txn, &account_key(from)).await;
                        let to_bal = get_balance(&txn, &account_key(to)).await;
                        let amount = rng.random_range(1..=100u64).min(from_bal);
                        if amount == 0 {
                            break;
                        }
                        txn.put(account_key(from), (from_bal - amount).to_le_bytes())
                            .unwrap();
                        txn.put(account_key(to), (to_bal + amount).to_le_bytes())
                            .unwrap();
                        match txn.commit().await {
                            Ok(_) => break,
                            Err(e) if e.kind() == slatedb::ErrorKind::Transaction => continue,
                            Err(e) => panic!("unexpected error: {e:?}"),
                        }
                    }
                }
            })
        })
        .collect();

    futures::future::try_join_all(handles).await.unwrap();

    let mut total = 0u64;
    for i in 0..num_accounts {
        let bal = db.get(&account_key(i)).await.unwrap().unwrap();
        total += u64::from_le_bytes(bal.as_ref().try_into().unwrap());
    }

    assert_eq!(
        total, expected_total,
        "snapshot isolation violation: total={total}, expected {expected_total}",
    );

    db.close().await.unwrap();
}
