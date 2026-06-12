use crate::db::BumpSubSubject;

#[deprecated(note = "pass BumpSubSubject directly to universalpubsub")]
#[allow(dead_code)]
pub fn convert(subject: BumpSubSubject) -> String {
	subject.to_string()
}
