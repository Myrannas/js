use crate::result::JsResult;
use crate::values::value::RuntimeValue;
use crate::{JsObject, JsThread};
use builtin::{callable, named, prototype};

pub(crate) struct JsError<'a, 'b> {
    object: &'b JsObject<'a>,
    thread: &'b mut JsThread<'a>,
}

#[prototype]
impl<'a, 'b> JsError<'a, 'b> {
    #[callable]
    fn constructor(&mut self, is_new: bool, message: RuntimeValue<'a>) {
        self.object.define_value("message", message.clone());
    }

    #[named("toString")]
    fn as_string(&mut self) -> JsResult<'a> {
        let message = self.object.get("message".into(), self.thread)?;

        Ok(message.to_string(self.thread)?.into())
    }
}