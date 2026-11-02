use super::*;
use crate::{CompactionFilterError, ValueDeletable};
use bytes::Bytes;
use std::collections::VecDeque;

struct Input(VecDeque<RowEntry>);
#[async_trait]
impl RowEntryIterator for Input {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        Ok(())
    }
    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        Ok(self.0.pop_front())
    }
    async fn seek(&mut self, key: &[u8]) -> Result<(), SlateDBError> {
        while self.0.front().is_some_and(|row| row.key.as_ref() < key) {
            self.0.pop_front();
        }
        Ok(())
    }
}
struct KeepFirst {
    previous: Option<u8>,
}
#[async_trait]
impl CompactionFilter for KeepFirst {
    async fn filter(
        &mut self,
        row: &RowEntry,
    ) -> Result<CompactionFilterDecision, CompactionFilterError> {
        let same = self.previous == Some(row.key[0]);
        self.previous = Some(row.key[0]);
        Ok(if same {
            CompactionFilterDecision::Drop
        } else {
            CompactionFilterDecision::Keep
        })
    }
    async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
        Ok(())
    }
}
#[tokio::test]
async fn compaction_filter_resume_reconstructs_history_before_the_cursor() {
    let rows = [b"a0", b"a1", b"a2", b"b0"]
        .into_iter()
        .map(|key| RowEntry {
            key: Bytes::copy_from_slice(key),
            value: ValueDeletable::Value(Bytes::new()),
            seq: 1,
            create_ts: None,
            expire_ts: None,
        })
        .collect();
    let mut iterator =
        CompactionFilterIterator::new(Input(rows), Box::new(KeepFirst { previous: None }))
            .with_full_input(true);
    iterator.init().await.unwrap();
    iterator.seek(b"a1").await.unwrap();
    assert_eq!(iterator.next().await.unwrap().unwrap().key.as_ref(), b"b0");
    assert!(iterator.next().await.unwrap().is_none());
}
