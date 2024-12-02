// See https://github.com/moby/sys/blob/c0711cde08c8fa33857a2c28721659267f49b5e2/user/user.go

use anyhow::*;

// MARK: passwd
#[derive(Debug)]
pub struct User {
	pub name: String,
	pub uid: u32,
	pub gid: u32,
	pub home: String,
	pub shell: String,
}

pub fn read_passwd_file(passwd_file: &str) -> Result<Vec<User>> {
	let mut users = Vec::new();
	for line in passwd_file.lines() {
		if let Some(user) = parse_passwd_line(&line) {
			users.push(user);
		}
	}

	Ok(users)
}

fn parse_passwd_line(line: &str) -> Option<User> {
	let fields: Vec<&str> = line.split(':').collect();
	if fields.len() < 7 {
		return None;
	}

	let uid: u32 = fields[2].parse().ok()?;
	let gid: u32 = fields[3].parse().ok()?;

	Some(User {
		name: fields[0].to_string(),
		uid,
		gid,
		home: fields[5].to_string(),
		shell: fields[6].to_string(),
	})
}

// MARK: groups
#[derive(Debug)]
pub struct Group {
	pub name: String,
	pub gid: u32,
	pub user_list: Vec<String>,
}

pub fn read_group_file(group_file: &str) -> Result<Vec<Group>> {
	let mut groups = Vec::new();
	for line in group_file.lines() {
		if let Some(group) = parse_group_line(&line) {
			groups.push(group);
		}
	}

	Ok(groups)
}

fn parse_group_line(line: &str) -> Option<Group> {
	let fields: Vec<&str> = line.split(':').collect();
	if fields.len() < 4 {
		return None;
	}

	let gid: u32 = fields[2].parse().ok()?;
	let user_list: Vec<String> = fields[3].split(',').map(|s| s.to_string()).collect();

	Some(Group {
		name: fields[0].to_string(),
		gid,
		user_list,
	})
}
