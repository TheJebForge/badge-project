use std::cell::{Ref, RefCell, RefMut};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use egui::WidgetText;

#[derive(Clone, Debug)]
pub struct SharedString(pub Rc<RefCell<String>>);

impl PartialEq for SharedString {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ptr() == other.0.as_ptr()
    }
}

impl Eq for SharedString {}

impl SharedString {
    pub fn as_ref(&'_ self) -> Ref<'_, String> {
        self.0.borrow()
    }

    pub fn as_mut(&'_ self) -> RefMut<'_, String> {
        self.0.borrow_mut()
    }

    pub fn str_eq(&self, other: &str) -> bool {
        self.0.borrow().eq(other)
    }
}

impl Display for SharedString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = self.0.borrow();
        write!(f, "{str}")
    }
}

impl From<String> for SharedString {
    fn from(value: String) -> Self {
        Self(Rc::new(RefCell::new(value)))
    }
}

impl From<SharedString> for String {
    fn from(value: SharedString) -> Self {
        value.as_ref().clone()
    }
}

impl Into<WidgetText> for SharedString {
    fn into(self) -> WidgetText {
        WidgetText::Text(self.to_string())
    }
}

impl Into<WidgetText> for &SharedString {
    fn into(self) -> WidgetText {
        WidgetText::Text(self.to_string())
    }
}

impl Into<WidgetText> for &mut SharedString {
    fn into(self) -> WidgetText {
        WidgetText::Text(self.to_string())
    }
}

impl Hash for SharedString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

pub trait AsSharedString {
    fn shared(&self) -> SharedString;
}

impl<T: Display> AsSharedString for T {
    fn shared(&self) -> SharedString {
        self.to_string().into()
    }
}

pub trait MutableStringScope {
    fn refer<R>(&self, scope_fn: impl FnOnce(&String) -> R) -> R;
    fn mutate<R>(&mut self, scope_fn: impl FnOnce(&mut String) -> R) -> R;
}

impl MutableStringScope for String {
    fn refer<R>(&self, scope_fn: impl FnOnce(&String) -> R) -> R {
        scope_fn(self)
    }

    fn mutate<R>(&mut self, scope_fn: impl FnOnce(&mut String) -> R) -> R {
        scope_fn(self)
    }
}

impl MutableStringScope for SharedString {
    fn refer<R>(&self, scope_fn: impl FnOnce(&String) -> R) -> R {
        let lock = self.as_ref();
        scope_fn(&*lock)
    }

    fn mutate<R>(&mut self, scope_fn: impl FnOnce(&mut String) -> R) -> R {
        let mut lock = self.as_mut();
        scope_fn(&mut *lock)
    }
}