use crate::context::JsContext;
use crate::debugging::{DebugRepresentation, DebugWithRealm, Renderer};
use crate::primordials::{get_prototype_property, RuntimeHelpers};
use crate::result::JsResult;
use crate::values::object::JsObject;
use crate::values::string::JsPrimitiveString;
use crate::vm::JsThread;

use crate::object_pool::ObjectPointer;
use crate::values::nan::Value;
use crate::Realm;
use core::cmp::PartialEq;
use core::convert::From;
use core::fmt::{Debug, Formatter};
use core::option::Option;
use core::option::Option::{None, Some};
use core::result::Result::{Err, Ok};
use instruction_set::{Function, Instruction, Local, LocalInit};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum FunctionReference<'a> {
    Custom(CustomFunctionReference<'a>),
    BuiltIn(BuiltIn<'a>),
}

#[derive(Clone, Debug)]
pub struct CustomFunctionReference<'a> {
    pub parent_context: JsContext<'a>,
    pub function: JsFunction,
}

impl<'a> PartialEq for CustomFunctionReference<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.function == other.function && self.parent_context == other.parent_context
    }
}

#[derive(Clone)]
pub struct BuiltIn<'a> {
    pub op: BuiltinFn<'a>,
    pub context: Option<Value<'a>>,
}

impl<'a> PartialEq for BuiltIn<'a> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.op, &other.op) && self.context == other.context
    }
}

impl<'a> From<BuiltIn<'a>> for FunctionReference<'a> {
    fn from(builtin: BuiltIn<'a>) -> Self {
        FunctionReference::BuiltIn(builtin)
    }
}

impl<'a> BuiltIn<'a> {
    pub fn apply_return(
        &self,
        arguments: usize,
        thread: &mut JsThread<'a>,
        target: Value<'a>,
    ) -> JsResult<'a, Option<Value<'a>>> {
        let start_len = thread.stack.len() - arguments;
        let result = (self.op)(arguments, thread, target, self.context);
        thread.stack.truncate(start_len);

        result
    }
}

pub type BuiltinFn<'a> = fn(
    arguments: usize,
    frame: &mut JsThread<'a>,
    target: Value<'a>,
    context: Option<Value<'a>>,
) -> JsResult<'a, Option<Value<'a>>>;

impl<'a> Debug for BuiltIn<'a> {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl From<JsFunctionInner> for JsFunction {
    fn from(inner: JsFunctionInner) -> Self {
        JsFunction {
            inner: Rc::new(inner),
        }
    }
}

#[derive(Clone)]
pub struct JsFunction {
    inner: Rc<JsFunctionInner>,
}

#[derive(Clone)]
pub struct JsClass {
    construct: Option<JsFunction>,
    atoms: Vec<JsPrimitiveString>,
    name: JsPrimitiveString,
    methods: Vec<JsFunction>,
    static_methods: Vec<JsFunction>,
}

#[derive(Copy, Clone)]
pub(crate) enum Prototype<'a> {
    Object,
    Null,
    Custom(ObjectPointer<'a>),
}

impl JsClass {
    pub(crate) fn load<'a>(
        &self,
        context: &JsContext<'a>,
        thread: &mut JsThread<'a>,
        prototype: Prototype<'a>,
    ) -> JsResult<'a> {
        let constructor = if let Some(constructor) = &self.construct {
            Some(FunctionReference::Custom(CustomFunctionReference {
                function: constructor.clone(),
                parent_context: context.clone(),
            }))
        } else if let Prototype::Custom(obj) = prototype {
            let callable = obj
                .get_construct(&thread.realm.objects)
                .or_else(|| obj.get_callable(&thread.realm.objects));

            if let Some(function) = callable {
                Some(function.clone())
            } else {
                return Err(thread
                    .new_type_error(format!(
                        "Class extends value {:?} is not a constructor or null",
                        thread.debug_value(&obj)
                    ))
                    .into());
            }
        } else {
            None
        };

        let maybe_prototype = match &prototype {
            Prototype::Object => Some(thread.realm.wrappers.object),
            Prototype::Custom(obj) => {
                let pointer = get_prototype_property(&thread.realm, *obj);
                Some(pointer)
            }
            Prototype::Null => None,
        };

        let mut actual_prototype = JsObject::builder(&mut thread.realm.objects);
        if let Some(prototype) = maybe_prototype {
            actual_prototype.with_prototype(prototype);
        }
        let prototype = actual_prototype.build();

        let mut object_builder = JsObject::builder(&mut thread.realm.objects);

        if let Some(construct) = constructor {
            object_builder.with_construct(construct);
        }

        object_builder.with_property(thread.realm.constants.prototype, prototype);
        object_builder.with_name(self.name);
        object_builder.with_prototype(thread.realm.wrappers.function);

        let object = object_builder.build();

        for method in &self.methods {
            let function = thread.new_function(
                method.name(),
                FunctionReference::Custom(CustomFunctionReference {
                    function: method.clone(),
                    parent_context: context.clone(),
                }),
            );

            prototype.set(&mut thread.realm.objects, method.name(), function.into());
        }

        for method in &self.static_methods {
            let function = thread.new_function(
                method.name(),
                FunctionReference::Custom(CustomFunctionReference {
                    function: method.clone(),
                    parent_context: context.clone(),
                }),
            );

            object.set(&mut thread.realm.objects, method.name(), function.into());
        }

        Ok(object.into())
    }
}

pub(crate) struct JsFunctionInner {
    pub instructions: Vec<Instruction>,
    pub atoms: Vec<JsPrimitiveString>,
    pub functions: Vec<JsFunction>,
    pub classes: Vec<JsClass>,

    pub args_size: usize,
    pub local_size: usize,
    pub locals: Vec<Local>,
    pub locals_init: Vec<LocalInit>,

    #[allow(dead_code)]
    pub(crate) stack_size: usize,
    pub(crate) name: JsPrimitiveString,
}

impl<'a> Debug for JsFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsFunction").finish()
    }
}

impl<'a> DebugRepresentation<'a> for JsFunction {
    fn render(&self, f: &mut Renderer<'a, '_, '_, '_>) -> std::fmt::Result {
        f.function("fixme")?;

        f.start_internal("FUNCTION")?;

        f.internal_key("instructions")?;

        f.new_line()?;

        for instruction in &self.inner.instructions {
            f.formatter.write_fmt(format_args!("{:?}", instruction))?;
            f.new_line()?;
        }

        f.end_internal()?;

        Ok(())
    }
}

impl<'a> PartialEq for JsFunction {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<'a> JsFunction {
    #[must_use]
    pub fn load(function: Function, realm: &mut Realm<'a>) -> Self {
        let Function {
            instructions,
            atoms,
            functions,
            stack_size,
            name,
            local_size,
            locals,
            locals_init,
            args_size,
            classes,
        } = function;

        let atoms = atoms
            .into_iter()
            .map(|atom| realm.strings.intern(atom))
            .collect();
        let functions = functions
            .into_iter()
            .map(|f| JsFunction::load(f, realm))
            .collect();
        let classes = classes
            .into_iter()
            .map(|f| {
                let name = f.atoms[0].clone();
                JsClass {
                    construct: f.construct.map(|f| JsFunction::load(f, realm)),
                    atoms: f
                        .atoms
                        .into_iter()
                        .map(|atom| realm.strings.intern(atom))
                        .collect(),
                    name: realm.strings.intern(name),
                    methods: f
                        .methods
                        .into_iter()
                        .map(|f| JsFunction::load(f, realm))
                        .collect(),
                    static_methods: f
                        .static_methods
                        .into_iter()
                        .map(|f| JsFunction::load(f, realm))
                        .collect(),
                }
            })
            .collect();

        JsFunction {
            inner: Rc::new(JsFunctionInner {
                instructions,
                atoms,
                functions,
                local_size,
                args_size,
                locals,
                stack_size,
                locals_init,
                classes,
                name: name.map_or(realm.constants.empty_string, |n| realm.strings.intern(n)),
            }),
        }
    }

    #[must_use]
    pub fn name(&self) -> JsPrimitiveString {
        self.inner.name
    }

    pub(crate) fn child_function(&self, index: usize) -> &JsFunction {
        &self.inner.functions[index]
    }

    pub(crate) fn child_class(&self, index: usize) -> &JsClass {
        &self.inner.classes[index]
    }

    #[must_use]
    pub fn local_size(&self) -> usize {
        self.inner.local_size
    }

    #[must_use]
    pub fn args_size(&self) -> usize {
        self.inner.args_size
    }

    pub(crate) fn locals(&self) -> &Vec<Local> {
        &self.inner.locals
    }

    #[must_use]
    pub fn get_atom(&self, index: usize) -> JsPrimitiveString {
        self.inner.atoms[index]
    }

    pub(crate) fn atoms(&self) -> &Vec<JsPrimitiveString> {
        &self.inner.atoms
    }

    pub(crate) fn init(
        &self,
        realm: &mut Realm<'a>,
        parent_context: &JsContext<'a>,
    ) -> Vec<Value<'a>> {
        self.inner
            .locals_init
            .iter()
            .map(|local_init| match local_init {
                LocalInit::Constant(constant) => Value::from_constant(&self.inner.atoms, *constant),
                LocalInit::Function(function_id) => {
                    let function = self.inner.functions[*function_id].clone();
                    let function_reference = CustomFunctionReference {
                        function: function.clone(),
                        parent_context: parent_context.clone(),
                    };

                    realm
                        .new_function(
                            function.name(),
                            FunctionReference::Custom(function_reference),
                        )
                        .into()
                }
            })
            .collect()
    }

    pub(crate) fn instructions(&self) -> &Vec<Instruction> {
        &self.inner.instructions
    }
}
