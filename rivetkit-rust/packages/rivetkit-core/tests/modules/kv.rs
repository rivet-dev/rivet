use super::*;

pub(crate) fn new_in_memory() -> Kv {
	Kv::new_in_memory()
}

mod moved_tests {
	use std::{
		sync::{Arc, Condvar, Mutex, mpsc},
		time::Duration,
	};

	use crate::types::ListOpts;

	#[tokio::test]
	async fn in_memory_backend_supports_basic_crud_and_listing() {
		let kv = super::new_in_memory();

		kv.batch_put(&[(b"alpha".as_slice(), b"1".as_slice())])
			.await
			.expect("batch put should succeed");
		kv.batch_put(&[(b"beta".as_slice(), b"2".as_slice())])
			.await
			.expect("second batch put should succeed");

		let values = kv
			.batch_get(&[b"alpha".as_slice(), b"beta".as_slice()])
			.await
			.expect("batch get should succeed");
		assert_eq!(values, vec![Some(b"1".to_vec()), Some(b"2".to_vec())]);

		let prefix = kv
			.list_prefix(b"a", ListOpts::default())
			.await
			.expect("list prefix should succeed");
		assert_eq!(prefix, vec![(b"alpha".to_vec(), b"1".to_vec())]);

		let range = kv
			.list_range(
				b"alpha",
				b"gamma",
				ListOpts {
					reverse: true,
					limit: Some(1),
				},
			)
			.await
			.expect("list range should succeed");
		assert_eq!(range, vec![(b"beta".to_vec(), b"2".to_vec())]);

		kv.delete_range(b"alpha", b"beta")
			.await
			.expect("delete range should succeed");
		kv.batch_delete(&[b"beta".as_slice()])
			.await
			.expect("batch delete should succeed");

		let remaining = kv
			.batch_get(&[b"alpha".as_slice(), b"beta".as_slice()])
			.await
			.expect("batch get after deletes should succeed");
		assert_eq!(remaining, vec![None, None]);
	}

	#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
	async fn in_memory_delete_range_blocks_concurrent_put_until_delete_commits() {
		let kv = super::new_in_memory();
		kv.put(b"alpha-old", b"old")
			.await
			.expect("seed put should succeed");

		let delete_started = Arc::new((Mutex::new(false), Condvar::new()));
		let release_delete = Arc::new((Mutex::new(false), Condvar::new()));
		kv.test_set_delete_range_after_write_lock_hook({
			let delete_started = Arc::clone(&delete_started);
			let release_delete = Arc::clone(&release_delete);
			move || {
				let (started, started_cv) = &*delete_started;
				*started.lock().expect("delete-started lock poisoned") = true;
				started_cv.notify_one();

				let (release, release_cv) = &*release_delete;
				let released = release.lock().expect("delete-release lock poisoned");
				let _released = release_cv
					.wait_while(released, |released| !*released)
					.expect("delete-release lock poisoned");
			}
		});

		let delete_task = tokio::spawn({
			let kv = kv.clone();
			async move {
				kv.delete_range(b"alpha", b"beta")
					.await
					.expect("delete range should succeed");
			}
		});

		let (started, started_cv) = &*delete_started;
		let started = started.lock().expect("delete-started lock poisoned");
		let (started, _) = started_cv
			.wait_timeout_while(started, Duration::from_secs(2), |started| !*started)
			.expect("delete-started lock poisoned");
		assert!(
			*started,
			"delete_range should reach the write-locked section"
		);
		drop(started);

		let (put_attempted_tx, put_attempted_rx) = mpsc::channel();
		let (put_done_tx, put_done_rx) = mpsc::channel();
		let put_task = tokio::spawn({
			let kv = kv.clone();
			async move {
				put_attempted_tx
					.send(())
					.expect("put-attempted receiver should still be alive");
				kv.put(b"alpha-new", b"new")
					.await
					.expect("concurrent put should succeed");
				put_done_tx
					.send(())
					.expect("put-done receiver should still be alive");
			}
		});

		put_attempted_rx
			.recv_timeout(Duration::from_secs(2))
			.expect("concurrent put should start");
		assert!(
			put_done_rx.recv_timeout(Duration::from_millis(50)).is_err(),
			"concurrent put must not commit while delete_range holds the write lock",
		);

		let (release, release_cv) = &*release_delete;
		*release.lock().expect("delete-release lock poisoned") = true;
		release_cv.notify_one();

		delete_task.await.expect("delete task should not panic");
		put_task.await.expect("put task should not panic");
		put_done_rx
			.recv_timeout(Duration::from_secs(2))
			.expect("concurrent put should finish after delete_range commits");

		assert_eq!(
			kv.get(b"alpha-old")
				.await
				.expect("old key lookup should succeed"),
			None,
		);
		assert_eq!(
			kv.get(b"alpha-new")
				.await
				.expect("new key lookup should succeed"),
			Some(b"new".to_vec()),
		);
	}
}
