mod common;

#[test]
fn success() {
	let mut setup = common::setup("echo starting; sleep 1; echo exiting");
	assert_eq!(0, setup.child.wait().unwrap().code().unwrap());
	assert_eq!("starting", setup.msg_rx.recv().unwrap().message);
	assert_eq!("exiting", setup.msg_rx.recv().unwrap().message);
	assert!(setup.msg_rx.recv().is_err());
}

#[test]
fn exit_code() {
	let mut setup = common::setup("echo starting; exit 69");
	assert_eq!(69, setup.child.wait().unwrap().code().unwrap());
	assert_eq!("starting", setup.msg_rx.recv().unwrap().message);
	assert!(setup.msg_rx.recv().is_err());
}

#[test]
fn sigterm() {
	// Traps SIGTERM and exits with code 69. We sleep before exiting to ensure there's not a race
	// condition between the "goodbye world" and "Terminated" logs.
	let mut setup = common::setup(
		r#"
        echo hello world
        trap 'sleep 1; echo goodbye world; exit 69' TERM
        sleep infinity
        "#,
	);
	setup.signal_child("TERM");
	assert_eq!("hello world", setup.msg_rx.recv().unwrap().message);
	assert_eq!("Terminated", setup.msg_rx.recv().unwrap().message); // Outputted from runc
	assert_eq!("goodbye world", setup.msg_rx.recv().unwrap().message);
	assert!(setup.msg_rx.recv().is_err());
	assert_eq!(69, setup.child.wait().unwrap().code().unwrap());
}

#[test]
fn rate_limit() {
	let setup = common::setup(
		r#"
        i=0
        while true; do
            echo $i
            i=$((i+1))
        done
        "#,
	);
	loop {
		let msg = setup.msg_rx.recv().unwrap();
		if msg.message.contains("logs rate limited") {
			println!("found rate limit");
			break;
		}
	}
}
