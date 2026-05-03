use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use anyhow::{Result, bail};
use parking_lot::Mutex;
use tokio::sync::Notify;

use crate::conveyer::types::DatabaseBranchId;

use super::actions::{DepotFaultAction, MAX_FAULT_DELAY};
use super::points::{DepotFaultPoint, FaultBoundary};

#[derive(Debug, Default)]
pub struct DepotFaultController {
	inner: Arc<Mutex<DepotFaultControllerInner>>,
}

#[derive(Debug, Default)]
struct DepotFaultControllerInner {
	next_rule_id: u64,
	rules: Vec<DepotFaultRule>,
	replay: Vec<DepotFaultReplayEvent>,
	pauses: Vec<DepotFaultPauseEntry>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DepotFaultRuleId(u64);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DepotFaultContext {
	pub actor_id: Option<String>,
	pub database_id: Option<String>,
	pub database_branch_id: Option<DatabaseBranchId>,
	pub checkpoint: Option<String>,
	pub page_number: Option<u32>,
	pub shard_id: Option<u32>,
	pub seed: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepotFaultFired {
	pub rule_id: DepotFaultRuleId,
	pub point: DepotFaultPoint,
	pub action: DepotFaultAction,
	pub boundary: FaultBoundary,
	pub context: DepotFaultContext,
	pub invocation: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepotFaultReplayEvent {
	pub kind: DepotFaultReplayEventKind,
	pub rule_id: DepotFaultRuleId,
	pub point: DepotFaultPoint,
	pub action: DepotFaultAction,
	pub boundary: FaultBoundary,
	pub context: DepotFaultContext,
	pub invocation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DepotFaultReplayEventKind {
	Fired,
	ExpectedButUnfired,
}

#[derive(Clone, Debug)]
pub struct DepotFaultPauseHandle {
	checkpoint: String,
	state: Arc<DepotFaultPauseState>,
}

#[derive(Debug)]
struct DepotFaultRule {
	id: DepotFaultRuleId,
	point: DepotFaultPoint,
	scope: DepotFaultContext,
	invocation: DepotFaultInvocation,
	action: DepotFaultAction,
	expected: bool,
	seen_count: u64,
	fired_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DepotFaultInvocation {
	Any,
	Nth(u64),
}

#[derive(Debug)]
struct DepotFaultPauseEntry {
	checkpoint: String,
	state: Arc<DepotFaultPauseState>,
}

#[derive(Debug)]
struct DepotFaultPauseState {
	reached: AtomicBool,
	released: AtomicBool,
	reached_notify: Notify,
	release_notify: Notify,
}

#[must_use]
pub struct DepotFaultRuleBuilder<'a> {
	controller: &'a DepotFaultController,
	point: DepotFaultPoint,
	scope: DepotFaultContext,
	invocation: DepotFaultInvocation,
	expected: bool,
}

impl DepotFaultController {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn at(&self, point: DepotFaultPoint) -> DepotFaultRuleBuilder<'_> {
		DepotFaultRuleBuilder {
			controller: self,
			point,
			scope: DepotFaultContext::default(),
			invocation: DepotFaultInvocation::Any,
			expected: true,
		}
	}

	pub async fn maybe_fire(
		&self,
		point: DepotFaultPoint,
		context: DepotFaultContext,
	) -> Result<Option<DepotFaultFired>> {
		let fired = {
			let mut inner = self.inner.lock();
			let mut fired = None;
			for rule in &mut inner.rules {
				if rule.point != point || !context_matches(&rule.scope, &context) {
					continue;
				}

				rule.seen_count += 1;
				if fired.is_none() && rule.should_fire_now() {
					rule.fired_count += 1;
					fired = Some(DepotFaultFired {
						rule_id: rule.id,
						point: rule.point.clone(),
						action: rule.action.clone(),
						boundary: rule.point.boundary(),
						context: context.clone(),
						invocation: rule.seen_count,
					});
				}
			}

			let Some(fired) = fired else {
				return Ok(None);
			};

			inner.replay.push(fired.replay_event());
			fired
		};

		match &fired.action {
			DepotFaultAction::Fail { message } => {
				bail!("injected depot fault at {:?}: {message}", fired.point);
			}
			DepotFaultAction::Pause { checkpoint } => {
				let pause = self.pause_handle(checkpoint);
				pause.pause_until_released().await;
				Ok(Some(fired))
			}
			DepotFaultAction::Delay { duration } => {
				validate_delay(*duration)?;
				tokio::time::sleep(*duration).await;
				Ok(Some(fired))
			}
			DepotFaultAction::DropArtifact => Ok(Some(fired)),
		}
	}

	pub fn pause_handle(&self, checkpoint: impl Into<String>) -> DepotFaultPauseHandle {
		let checkpoint = checkpoint.into();
		let state = {
			let mut inner = self.inner.lock();
			inner.pause_state(&checkpoint)
		};

		DepotFaultPauseHandle { checkpoint, state }
	}

	pub fn replay_log(&self) -> Vec<DepotFaultReplayEvent> {
		self.inner.lock().replay.clone()
	}

	pub fn replay_log_with_unfired(&self) -> Vec<DepotFaultReplayEvent> {
		let inner = self.inner.lock();
		let mut replay = inner.replay.clone();
		for rule in inner
			.rules
			.iter()
			.filter(|rule| rule.expected && rule.fired_count == 0)
		{
			if replay.iter().any(|event| {
				event.rule_id == rule.id
					&& event.kind == DepotFaultReplayEventKind::ExpectedButUnfired
			}) {
				continue;
			}
			replay.push(rule.unfired_replay_event());
		}
		replay
	}

	pub fn assert_expected_fired(&self) -> Result<()> {
		let mut inner = self.inner.lock();
		inner.record_unfired_expected();
		let unfired = inner
			.rules
			.iter()
			.filter(|rule| rule.expected && rule.fired_count == 0)
			.map(|rule| format!("{:?} at {:?}", rule.id, rule.point))
			.collect::<Vec<_>>();

		if unfired.is_empty() {
			return Ok(());
		}

		bail!("expected depot faults did not fire: {}", unfired.join(", "))
	}
}

impl Clone for DepotFaultController {
	fn clone(&self) -> Self {
		Self {
			inner: Arc::clone(&self.inner),
		}
	}
}

impl DepotFaultRuleId {
	pub fn get(self) -> u64 {
		self.0
	}
}

impl DepotFaultContext {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn actor_id(mut self, actor_id: impl Into<String>) -> Self {
		self.actor_id = Some(actor_id.into());
		self
	}

	pub fn database_id(mut self, database_id: impl Into<String>) -> Self {
		self.database_id = Some(database_id.into());
		self
	}

	pub fn database_branch_id(mut self, database_branch_id: DatabaseBranchId) -> Self {
		self.database_branch_id = Some(database_branch_id);
		self
	}

	pub fn checkpoint(mut self, checkpoint: impl Into<String>) -> Self {
		self.checkpoint = Some(checkpoint.into());
		self
	}

	pub fn page_number(mut self, page_number: u32) -> Self {
		self.page_number = Some(page_number);
		self
	}

	pub fn shard_id(mut self, shard_id: u32) -> Self {
		self.shard_id = Some(shard_id);
		self
	}

	pub fn seed(mut self, seed: u64) -> Self {
		self.seed = Some(seed);
		self
	}
}

impl DepotFaultFired {
	fn replay_event(&self) -> DepotFaultReplayEvent {
		DepotFaultReplayEvent {
			kind: DepotFaultReplayEventKind::Fired,
			rule_id: self.rule_id,
			point: self.point.clone(),
			action: self.action.clone(),
			boundary: self.boundary,
			context: self.context.clone(),
			invocation: self.invocation,
		}
	}
}

impl DepotFaultRule {
	fn should_fire_now(&self) -> bool {
		match self.invocation {
			DepotFaultInvocation::Any => true,
			DepotFaultInvocation::Nth(invocation) => self.seen_count == invocation,
		}
	}

	fn unfired_replay_event(&self) -> DepotFaultReplayEvent {
		DepotFaultReplayEvent {
			kind: DepotFaultReplayEventKind::ExpectedButUnfired,
			rule_id: self.id,
			point: self.point.clone(),
			action: self.action.clone(),
			boundary: self.point.boundary(),
			context: self.scope.clone(),
			invocation: self.seen_count,
		}
	}
}

impl DepotFaultControllerInner {
	fn insert_rule(
		&mut self,
		point: DepotFaultPoint,
		scope: DepotFaultContext,
		invocation: DepotFaultInvocation,
		action: DepotFaultAction,
		expected: bool,
	) -> Result<DepotFaultRuleId> {
		if let DepotFaultAction::Delay { duration } = action {
			validate_delay(duration)?;
		}

		let id = DepotFaultRuleId(self.next_rule_id);
		self.next_rule_id += 1;
		self.rules.push(DepotFaultRule {
			id,
			point,
			scope,
			invocation,
			action,
			expected,
			seen_count: 0,
			fired_count: 0,
		});
		Ok(id)
	}

	fn pause_state(&mut self, checkpoint: &str) -> Arc<DepotFaultPauseState> {
		if let Some(entry) = self
			.pauses
			.iter()
			.find(|entry| entry.checkpoint == checkpoint)
		{
			return Arc::clone(&entry.state);
		}

		let state = Arc::new(DepotFaultPauseState::new());
		self.pauses.push(DepotFaultPauseEntry {
			checkpoint: checkpoint.to_string(),
			state: Arc::clone(&state),
		});
		state
	}

	fn record_unfired_expected(&mut self) {
		let unfired = self
			.rules
			.iter()
			.filter(|rule| rule.expected && rule.fired_count == 0)
			.filter(|rule| {
				!self.replay.iter().any(|event| {
					event.rule_id == rule.id
						&& event.kind == DepotFaultReplayEventKind::ExpectedButUnfired
				})
			})
			.map(DepotFaultRule::unfired_replay_event)
			.collect::<Vec<_>>();
		self.replay.extend(unfired);
	}
}

impl DepotFaultPauseHandle {
	pub fn checkpoint(&self) -> &str {
		&self.checkpoint
	}

	pub async fn wait_reached(&self) {
		while !self.state.reached.load(Ordering::SeqCst) {
			self.state.reached_notify.notified().await;
		}
	}

	pub fn release(&self) {
		self.state.released.store(true, Ordering::SeqCst);
		self.state.release_notify.notify_waiters();
	}

	async fn pause_until_released(&self) {
		self.state.reached.store(true, Ordering::SeqCst);
		self.state.reached_notify.notify_waiters();

		while !self.state.released.load(Ordering::SeqCst) {
			self.state.release_notify.notified().await;
		}
	}
}

impl DepotFaultPauseState {
	fn new() -> Self {
		Self {
			reached: AtomicBool::new(false),
			released: AtomicBool::new(false),
			reached_notify: Notify::new(),
			release_notify: Notify::new(),
		}
	}
}

impl<'a> DepotFaultRuleBuilder<'a> {
	pub fn actor_id(mut self, actor_id: impl Into<String>) -> Self {
		self.scope.actor_id = Some(actor_id.into());
		self
	}

	pub fn database_id(mut self, database_id: impl Into<String>) -> Self {
		self.scope.database_id = Some(database_id.into());
		self
	}

	pub fn database_branch_id(mut self, database_branch_id: DatabaseBranchId) -> Self {
		self.scope.database_branch_id = Some(database_branch_id);
		self
	}

	pub fn checkpoint(mut self, checkpoint: impl Into<String>) -> Self {
		self.scope.checkpoint = Some(checkpoint.into());
		self
	}

	pub fn page_number(mut self, page_number: u32) -> Self {
		self.scope.page_number = Some(page_number);
		self
	}

	pub fn shard_id(mut self, shard_id: u32) -> Self {
		self.scope.shard_id = Some(shard_id);
		self
	}

	pub fn seed(mut self, seed: u64) -> Self {
		self.scope.seed = Some(seed);
		self
	}

	pub fn once(mut self) -> Self {
		self.invocation = DepotFaultInvocation::Nth(1);
		self
	}

	pub fn nth(mut self, invocation: u64) -> Self {
		self.invocation = DepotFaultInvocation::Nth(invocation);
		self
	}

	pub fn optional(mut self) -> Self {
		self.expected = false;
		self
	}

	pub fn fail(self, message: impl Into<String>) -> Result<DepotFaultRuleId> {
		self.insert(DepotFaultAction::Fail {
			message: message.into(),
		})
	}

	pub fn pause(self, checkpoint: impl Into<String>) -> Result<DepotFaultRuleId> {
		let checkpoint = checkpoint.into();
		self.controller.pause_handle(checkpoint.clone());
		self.insert(DepotFaultAction::Pause { checkpoint })
	}

	pub fn delay(self, duration: Duration) -> Result<DepotFaultRuleId> {
		self.insert(DepotFaultAction::Delay { duration })
	}

	pub fn drop_artifact(self) -> Result<DepotFaultRuleId> {
		self.insert(DepotFaultAction::DropArtifact)
	}

	fn insert(self, action: DepotFaultAction) -> Result<DepotFaultRuleId> {
		self.controller.inner.lock().insert_rule(
			self.point,
			self.scope,
			self.invocation,
			action,
			self.expected,
		)
	}
}

fn context_matches(rule_scope: &DepotFaultContext, context: &DepotFaultContext) -> bool {
	optional_matches(&rule_scope.actor_id, &context.actor_id)
		&& optional_matches(&rule_scope.database_id, &context.database_id)
		&& optional_matches(&rule_scope.database_branch_id, &context.database_branch_id)
		&& optional_matches(&rule_scope.checkpoint, &context.checkpoint)
		&& optional_matches(&rule_scope.page_number, &context.page_number)
		&& optional_matches(&rule_scope.shard_id, &context.shard_id)
		&& optional_matches(&rule_scope.seed, &context.seed)
}

fn optional_matches<T: PartialEq>(expected: &Option<T>, actual: &Option<T>) -> bool {
	match expected {
		Some(expected) => actual.as_ref() == Some(expected),
		None => true,
	}
}

fn validate_delay(duration: Duration) -> Result<()> {
	if duration <= MAX_FAULT_DELAY {
		return Ok(());
	}

	bail!(
		"depot fault delay {:?} exceeds maximum {:?}",
		duration,
		MAX_FAULT_DELAY
	)
}
