use super::builtins::prototype::Prototype;
use super::builtins::{array, errors, function, number, objects, string};
use crate::function::FunctionReference;
use crate::values::object::JsObject;
use crate::values::string::JsPrimitiveString;
use crate::values::value::RuntimeValue;
use crate::BuiltIn;

trait Helpers<'a> {
    fn define_readonly_value<S: Into<String>, V: Into<RuntimeValue<'a>>>(&self, key: S, value: V);
}

impl<'a> Helpers<'a> for JsObject<'a> {
    fn define_readonly_value<S: Into<String>, V: Into<RuntimeValue<'a>>>(&self, key: S, value: V) {
        self.define_property(
            key.into(),
            Some(FunctionReference::BuiltIn(BuiltIn {
                context: Some(Box::new(value.into())),
                op: |_, _thread, _, context| Ok(Some(context.unwrap().clone())),
            })),
            None,
        )
    }
}

#[derive(Clone)]
pub(crate) struct Errors<'a> {
    reference_error: JsObject<'a>,
    syntax_error: JsObject<'a>,
    type_error: JsObject<'a>,
    error: JsObject<'a>,
}

#[derive(Clone)]
pub(crate) struct Primitives<'a> {
    string: JsObject<'a>,
    function: JsObject<'a>,
    number: JsObject<'a>,
    object: JsObject<'a>,
    array: JsObject<'a>,
}

#[derive(Clone)]
pub struct GlobalThis<'a> {
    pub(crate) global_this: JsObject<'a>,
    pub(crate) errors: Errors<'a>,
    pub(crate) wrappers: Primitives<'a>,
}

impl<'a> Default for GlobalThis<'a> {
    fn default() -> Self {
        GlobalThis::new()
    }
}

impl<'a> GlobalThis<'a> {
    pub fn new() -> GlobalThis<'a> {
        let global_this = JsObject::new();

        let primitives = Primitives::init(&global_this);
        let errors = Errors::init(&global_this);

        global_this.define_value("Math", super::builtins::math::JsMath::bind(None));

        #[cfg(feature = "eval")]
        {
            global_this.define_readonly_value(
                "eval",
                primitives.wrap_function(BuiltIn {
                    context: None,
                    op: super::builtins::eval::eval,
                }),
            );
        }
        global_this.define_readonly_value("undefined", RuntimeValue::Undefined);
        global_this.define_readonly_value("NaN", f64::NAN);

        GlobalThis {
            global_this,
            wrappers: primitives,
            errors,
        }
    }
}

impl<'a> Primitives<'a> {
    fn init(global_this: &JsObject<'a>) -> Primitives<'a> {
        let object_prototype: JsObject<'a> = objects::JsObjectBase::bind(None);

        let string_prototype: JsObject<'a> = string::JsString::bind(Some(&object_prototype));
        let number_prototype: JsObject<'a> = number::JsNumber::bind(Some(&object_prototype));
        let array_prototype: JsObject<'a> = array::JsArray::bind(Some(&object_prototype));

        let function_prototype = function::JsFunctionObject::bind(Some(&object_prototype));

        let primitives = Primitives {
            string: string_prototype.clone(),
            function: function_prototype.clone(),
            object: object_prototype.clone(),
            array: array_prototype.clone(),
            number: number_prototype.clone(),
        };

        global_this.define_value("String", string_prototype);
        global_this.define_value("Array", array_prototype);
        global_this.define_value("Object", object_prototype);
        global_this.define_value("Function", function_prototype);
        global_this.define_value("Number", number_prototype.clone());
        global_this.define_value(
            "parseInt",
            number_prototype.read_simple_property("parseInt"),
        );

        primitives
    }

    pub(crate) fn wrap_string(&self, string: JsPrimitiveString) -> JsObject<'a> {
        JsObject::new()
            .wrapping(RuntimeValue::String(string))
            .with_prototype(self.string.clone())
    }

    pub(crate) fn wrap_number(&self, number: f64) -> JsObject<'a> {
        JsObject::new()
            .wrapping(RuntimeValue::Float(number))
            .with_prototype(self.number.clone())
    }

    pub(crate) fn wrap_boolean(&self, number: bool) -> JsObject<'a> {
        JsObject::new()
            .wrapping(RuntimeValue::Boolean(number))
            .with_prototype(self.number.clone()) // todo: Fixme
    }

    pub(crate) fn wrap_function(&self, function: impl Into<FunctionReference<'a>>) -> JsObject<'a> {
        JsObject::new()
            .callable(function)
            .with_prototype(self.function.clone())
    }

    pub fn new_object(&self) -> JsObject<'a> {
        JsObject::new().with_prototype(self.object.clone())
    }
}

impl<'a> Errors<'a> {
    #[allow(dead_code)]
    pub(crate) fn new_reference_error(&self, message: impl Into<String>) -> RuntimeValue<'a> {
        self.new_error(&self.reference_error, message.into()).into()
    }

    #[allow(dead_code)]
    pub(crate) fn new_syntax_error(&self, message: impl Into<String>) -> RuntimeValue<'a> {
        self.new_error(&self.syntax_error, message.into()).into()
    }

    pub(crate) fn new_type_error(&self, message: impl Into<String>) -> RuntimeValue<'a> {
        self.new_error(&self.type_error, message.into()).into()
    }

    fn new_error(&self, prototype: &JsObject<'a>, message: impl Into<String>) -> JsObject<'a> {
        let error = JsObject::new().with_prototype(prototype.clone());

        error.set("message".into(), message.into());

        error
    }

    fn init(global_this: &JsObject<'a>) -> Errors<'a> {
        let error = errors::JsError::bind(None);

        let reference_error = JsObject::new().with_prototype(error.clone());
        let syntax_error = JsObject::new().with_prototype(error.clone());
        let type_error = JsObject::new().with_prototype(error.clone());

        global_this.define_readonly_value("ReferenceError", reference_error.clone());
        global_this.define_readonly_value("SyntaxError", syntax_error.clone());
        global_this.define_readonly_value("TypeError", type_error.clone());
        global_this.define_readonly_value("Error", error.clone());

        Errors {
            syntax_error,
            reference_error,
            type_error,
            error,
        }
    }
}