use crate::object_pool::ObjectPointer;
use crate::string_pool::StringPointer;
use crate::JsPrimitiveString;
use std::marker::PhantomData;

#[derive(Copy, Clone)]
struct Value<'a> {
    inner: u64,
    phantom: PhantomData<ObjectPointer<'a>>,
}

#[derive(Copy, Clone)]
enum ValueType<'a> {
    Float,
    FloatNaN,
    Object(ObjectPointer<'a>),
    String(JsPrimitiveString),
    Null,
    Undefined,
    Boolean(bool),
    Local(u32),
    StringReference(JsPrimitiveString),
    NumberReference(u32),
}

const DOWNSHIFT: u64 = 47;
const FLOAT_NAN_TAG: u64 = 9221120237041090560u64 >> DOWNSHIFT;
const OBJECT_TAG: u64 = FLOAT_NAN_TAG + 1;
const NULL_TAG: u64 = FLOAT_NAN_TAG + 2;
const UNDEFINED_TAG: u64 = FLOAT_NAN_TAG + 3;
const STRING_TAG: u64 = FLOAT_NAN_TAG + 4;
const BOOLEAN_TRUE_TAG: u64 = FLOAT_NAN_TAG + 5;
const BOOLEAN_FALSE_TAG: u64 = FLOAT_NAN_TAG + 6;
const LOCAL_TAG: u64 = FLOAT_NAN_TAG + 7;
const STRING_REFERENCE_TAG: u64 = FLOAT_NAN_TAG + 8;
const NUMBER_REFERENCE_TAG: u64 = FLOAT_NAN_TAG + 9;

impl<'a> Value<'a> {
    fn get_type(self) -> ValueType<'a> {
        let tag = self.inner >> 48;

        match tag {
            FLOAT_NAN_TAG => ValueType::FloatNaN,
            OBJECT_TAG => ValueType::Object(ObjectPointer::new(self.inner as u32)),
            NULL_TAG => ValueType::Null,
            UNDEFINED_TAG => ValueType::Undefined,
            STRING_TAG => ValueType::String(StringPointer::new(self.inner as u32)),
            BOOLEAN_TRUE_TAG => ValueType::Boolean(true),
            BOOLEAN_FALSE_TAG => ValueType::Boolean(false),
            LOCAL_TAG => ValueType::Local(tag as u32),
            STRING_REFERENCE_TAG => {
                ValueType::StringReference(StringPointer::new(self.inner as u32))
            }
            NUMBER_REFERENCE_TAG => ValueType::NumberReference(self.inner as u32),
            _ => ValueType::Float,
        }
    }

    fn float(self) -> f64 {
        f64::from_bits(self.inner)
    }
}

#[cfg(test)]
mod test {
    use crate::values::nan::{Value, ValueType};

    #[test]
    fn test() {
        assert_eq!(std::mem::size_of::<Value>(), 8);
        assert_eq!(std::mem::size_of::<ValueType>(), 8);
    }
}
