//! Typed properties, property values and option/page references.

use serde::{Deserialize, Serialize};

use super::database::DataSource;
use super::page::Page;
use super::rich_text::RichText;
use super::user::UserRef;
/// A data-source property type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    Title,
    RichText,
    Number,
    Select,
    MultiSelect,
    Status,
    Date,
    Checkbox,
    Url,
    Email,
    PhoneNumber,
    People,
    Files,
    Relation,
    Formula,
    Rollup,
    CreatedTime,
    CreatedBy,
    LastEditedTime,
    LastEditedBy,
    UniqueId,
}

impl PropertyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PropertyType::Title => "title",
            PropertyType::RichText => "rich_text",
            PropertyType::Number => "number",
            PropertyType::Select => "select",
            PropertyType::MultiSelect => "multi_select",
            PropertyType::Status => "status",
            PropertyType::Date => "date",
            PropertyType::Checkbox => "checkbox",
            PropertyType::Url => "url",
            PropertyType::Email => "email",
            PropertyType::PhoneNumber => "phone_number",
            PropertyType::People => "people",
            PropertyType::Files => "files",
            PropertyType::Relation => "relation",
            PropertyType::Formula => "formula",
            PropertyType::Rollup => "rollup",
            PropertyType::CreatedTime => "created_time",
            PropertyType::CreatedBy => "created_by",
            PropertyType::LastEditedTime => "last_edited_time",
            PropertyType::LastEditedBy => "last_edited_by",
            PropertyType::UniqueId => "unique_id",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectOption {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub description: Option<String>,
}

impl SelectOption {
    pub fn new(id: impl Into<String>, name: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            color: color.into(),
            description: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub option_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PropertyConfig {
    #[default]
    None,
    Number {
        format: String,
    },
    Select {
        options: Vec<SelectOption>,
    },
    MultiSelect {
        options: Vec<SelectOption>,
    },
    Status {
        default_option: Option<String>,
        options: Vec<SelectOption>,
        groups: Vec<StatusGroup>,
    },
    Relation {
        data_source_id: String,
        single_property: bool,
    },
    Formula {
        expression: String,
    },
    Rollup {
        relation_property_id: String,
        rollup_property_id: String,
        function: String,
    },
    UniqueId {
        prefix: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub r#type: PropertyType,
    #[serde(default)]
    pub config: PropertyConfig,
}

impl Property {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        r#type: PropertyType,
        config: PropertyConfig,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            r#type,
            config,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateValue {
    pub start: String,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
}

impl DateValue {
    pub fn new(start: impl Into<String>) -> Self {
        Self {
            start: start.into(),
            end: None,
            time_zone: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionRef {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRef {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRef {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub expiry_time: Option<String>,
    #[serde(default)]
    pub external: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormulaResult {
    String(Option<String>),
    Number(Option<i64>),
    Boolean(bool),
    Date(Option<DateValue>),
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RollupResult {
    Number(Option<i64>),
    Date(Option<DateValue>),
    Array(Vec<PropertyValue>),
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyValue {
    Title(Vec<RichText>),
    RichText(Vec<RichText>),
    Number(Option<i64>),
    Select(Option<OptionRef>),
    MultiSelect(Vec<OptionRef>),
    Status(Option<OptionRef>),
    Date(Option<DateValue>),
    Checkbox(bool),
    Url(Option<String>),
    Email(Option<String>),
    PhoneNumber(Option<String>),
    People(Vec<UserRef>),
    Files(Vec<FileRef>),
    Relation(Vec<PageRef>),
    Formula(FormulaResult),
    Rollup(RollupResult),
    CreatedTime(String),
    CreatedBy(UserRef),
    LastEditedTime(String),
    LastEditedBy(UserRef),
    UniqueId { number: u64, prefix: Option<String> },
}

impl PropertyValue {
    /// Apply the v2 clear rule for a property type.
    pub fn clear(kind: PropertyType) -> Self {
        match kind {
            PropertyType::Title => PropertyValue::Title(Vec::new()),
            PropertyType::RichText => PropertyValue::RichText(Vec::new()),
            PropertyType::MultiSelect => PropertyValue::MultiSelect(Vec::new()),
            PropertyType::People => PropertyValue::People(Vec::new()),
            PropertyType::Files => PropertyValue::Files(Vec::new()),
            PropertyType::Relation => PropertyValue::Relation(Vec::new()),
            PropertyType::Checkbox => PropertyValue::Checkbox(false),
            PropertyType::Number => PropertyValue::Number(None),
            PropertyType::Select => PropertyValue::Select(None),
            PropertyType::Status => PropertyValue::Status(None),
            PropertyType::Date => PropertyValue::Date(None),
            PropertyType::Url => PropertyValue::Url(None),
            PropertyType::Email => PropertyValue::Email(None),
            PropertyType::PhoneNumber => PropertyValue::PhoneNumber(None),
            PropertyType::Formula => PropertyValue::Formula(FormulaResult::Unsupported),
            PropertyType::Rollup => PropertyValue::Rollup(RollupResult::Unsupported),
            PropertyType::CreatedTime => PropertyValue::CreatedTime(String::new()),
            PropertyType::CreatedBy => PropertyValue::CreatedBy(UserRef::new("")),
            PropertyType::LastEditedTime => PropertyValue::LastEditedTime(String::new()),
            PropertyType::LastEditedBy => PropertyValue::LastEditedBy(UserRef::new("")),
            PropertyType::UniqueId => PropertyValue::UniqueId {
                number: 0,
                prefix: None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageProperty {
    pub id: String,
    pub r#type: PropertyType,
    pub value: PropertyValue,
}

impl PageProperty {
    pub fn new(id: impl Into<String>, r#type: PropertyType, value: PropertyValue) -> Self {
        Self {
            id: id.into(),
            r#type,
            value,
        }
    }

    /// One item per value for list-like properties, one item otherwise.
    pub fn items(&self) -> Vec<PageProperty> {
        let make = |value: PropertyValue| PageProperty {
            id: self.id.clone(),
            r#type: self.r#type,
            value,
        };
        match &self.value {
            PropertyValue::Title(values) => values
                .iter()
                .map(|value| make(PropertyValue::Title(vec![value.clone()])))
                .collect(),
            PropertyValue::RichText(values) => values
                .iter()
                .map(|value| make(PropertyValue::RichText(vec![value.clone()])))
                .collect(),
            PropertyValue::People(values) => values
                .iter()
                .map(|value| make(PropertyValue::People(vec![value.clone()])))
                .collect(),
            PropertyValue::Relation(values) => values
                .iter()
                .map(|value| make(PropertyValue::Relation(vec![value.clone()])))
                .collect(),
            _ => vec![self.clone()],
        }
    }
}

pub(super) fn resolve_property<'a>(data_source: &'a DataSource, key: &str) -> Option<&'a Property> {
    data_source.properties.get(key).or_else(|| {
        data_source
            .properties
            .values()
            .find(|property| property.id == key)
    })
}

pub(super) fn value_as_string(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Title(values) | PropertyValue::RichText(values) => values
            .iter()
            .map(RichText::plain_text)
            .collect::<Vec<_>>()
            .join(""),
        PropertyValue::Number(value) => value.map(|n| n.to_string()).unwrap_or_default(),
        PropertyValue::Select(option) | PropertyValue::Status(option) => option
            .as_ref()
            .map(|option| option.name.clone())
            .unwrap_or_default(),
        PropertyValue::MultiSelect(options) => options
            .iter()
            .map(|option| option.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        PropertyValue::Date(date) => date
            .as_ref()
            .map(|date| date.start.clone())
            .unwrap_or_default(),
        PropertyValue::Checkbox(value) => value.to_string(),
        PropertyValue::Url(value)
        | PropertyValue::Email(value)
        | PropertyValue::PhoneNumber(value) => value.clone().unwrap_or_default(),
        PropertyValue::People(users) => users
            .iter()
            .map(|user| user.id.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        PropertyValue::Files(files) => files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        PropertyValue::Relation(pages) => pages
            .iter()
            .map(|page| page.id.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        PropertyValue::Formula(result) => match result {
            FormulaResult::String(value) => value.clone().unwrap_or_default(),
            FormulaResult::Number(value) => value.map(|n| n.to_string()).unwrap_or_default(),
            FormulaResult::Boolean(value) => value.to_string(),
            FormulaResult::Date(value) => value
                .as_ref()
                .map(|date| date.start.clone())
                .unwrap_or_default(),
            FormulaResult::Unsupported => String::new(),
        },
        PropertyValue::Rollup(result) => match result {
            RollupResult::Number(value) => value.map(|n| n.to_string()).unwrap_or_default(),
            RollupResult::Date(value) => value
                .as_ref()
                .map(|date| date.start.clone())
                .unwrap_or_default(),
            RollupResult::Array(values) => values
                .iter()
                .map(value_as_string)
                .collect::<Vec<_>>()
                .join(", "),
            RollupResult::Unsupported => String::new(),
        },
        PropertyValue::CreatedTime(value) | PropertyValue::LastEditedTime(value) => value.clone(),
        PropertyValue::CreatedBy(user) | PropertyValue::LastEditedBy(user) => user.id.clone(),
        PropertyValue::UniqueId { number, prefix } => {
            format!("{}{number}", prefix.clone().unwrap_or_default())
        }
    }
}

pub(super) fn value_is_empty(value: &PropertyValue) -> bool {
    match value {
        PropertyValue::Title(values) | PropertyValue::RichText(values) => values.is_empty(),
        PropertyValue::MultiSelect(values) => values.is_empty(),
        PropertyValue::People(values) => values.is_empty(),
        PropertyValue::Files(values) => values.is_empty(),
        PropertyValue::Relation(values) => values.is_empty(),
        PropertyValue::Number(value) => value.is_none(),
        PropertyValue::Select(value) | PropertyValue::Status(value) => value.is_none(),
        PropertyValue::Date(value) => value.is_none(),
        PropertyValue::Checkbox(value) => !value,
        PropertyValue::Url(value)
        | PropertyValue::Email(value)
        | PropertyValue::PhoneNumber(value) => value.as_deref().is_none_or(str::is_empty),
        PropertyValue::Formula(FormulaResult::Unsupported)
        | PropertyValue::Rollup(RollupResult::Unsupported) => true,
        _ => value_as_string(value).is_empty(),
    }
}

pub(super) fn value_as_number(value: &PropertyValue) -> Option<i64> {
    match value {
        PropertyValue::Number(value) => *value,
        PropertyValue::Formula(FormulaResult::Number(value)) => *value,
        PropertyValue::Rollup(RollupResult::Number(value)) => *value,
        PropertyValue::UniqueId { number, .. } => i64::try_from(*number).ok(),
        _ => value_as_string(value).parse().ok(),
    }
}

pub(super) fn property_value<'a>(
    data_source: &DataSource,
    row: &'a Page,
    property: &str,
) -> Option<&'a PropertyValue> {
    let property = resolve_property(data_source, property)?;
    row.properties
        .get(&property.id)
        .map(|property| &property.value)
}
