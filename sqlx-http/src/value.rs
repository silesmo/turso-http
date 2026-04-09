use std::borrow::Cow;

use sqlx_core::value::{Value, ValueRef};

use crate::db::HttpDb;
use crate::type_info::HttpTypeInfo;

pub struct HttpValue {
    pub(crate) value: serde_json::Value,
    pub(crate) type_info: HttpTypeInfo,
}

impl Value for HttpValue {
    type Database = HttpDb;

    fn as_ref(&self) -> HttpValueRef<'_> {
        HttpValueRef {
            value: &self.value,
            type_info: self.type_info.clone(),
        }
    }

    fn type_info(&self) -> Cow<'_, HttpTypeInfo> {
        Cow::Borrowed(&self.type_info)
    }

    fn is_null(&self) -> bool {
        self.value.is_null()
    }
}

pub struct HttpValueRef<'r> {
    pub(crate) value: &'r serde_json::Value,
    pub(crate) type_info: HttpTypeInfo,
}

impl<'r> ValueRef<'r> for HttpValueRef<'r> {
    type Database = HttpDb;

    fn to_owned(&self) -> HttpValue {
        HttpValue {
            value: self.value.clone(),
            type_info: self.type_info.clone(),
        }
    }

    fn type_info(&self) -> Cow<'_, HttpTypeInfo> {
        Cow::Owned(self.type_info.clone())
    }

    fn is_null(&self) -> bool {
        self.value.is_null()
    }
}
