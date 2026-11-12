//! Saved views, filters, sorts and grouped query results.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use super::database::DataSource;
use super::dto::ParentDto;
use super::page::Page;
use super::property::{
    PropertyValue, property_value, value_as_number, value_as_string, value_is_empty,
};
use super::user::UserRef;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ViewType {
    #[default]
    Table,
    Board,
    Calendar,
    List,
    Gallery,
    Timeline,
    Form,
    Chart,
    Map,
    Dashboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sort {
    pub property: String,
    pub direction: SortDirection,
}

impl Sort {
    pub fn new(property: impl Into<String>, direction: SortDirection) -> Self {
        Self {
            property: property.into(),
            direction,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operator", rename_all = "snake_case")]
pub enum Filter {
    And { filters: Vec<Filter> },
    Or { filters: Vec<Filter> },
    Equals { property: String, value: String },
    NotEquals { property: String, value: String },
    Contains { property: String, value: String },
    IsEmpty { property: String },
    IsNotEmpty { property: String },
    GreaterThan { property: String, value: i64 },
    LessThan { property: String, value: i64 },
    Checked { property: String, value: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ViewConfig {
    #[default]
    Table,
    Board {
        group_by: String,
    },
    Calendar {
        date_property: String,
    },
    List,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct View {
    pub id: String,
    pub parent: ParentDto,
    #[serde(default)]
    pub data_source_id: Option<String>,
    pub name: String,
    pub r#type: ViewType,
    #[serde(default)]
    pub filter: Option<Filter>,
    #[serde(default)]
    pub sorts: Option<Vec<Sort>>,
    #[serde(default)]
    pub configuration: Option<ViewConfig>,
    #[serde(default)]
    pub created_time: String,
    #[serde(default)]
    pub created_by: Option<UserRef>,
    #[serde(default)]
    pub last_edited_time: String,
    #[serde(default)]
    pub last_edited_by: Option<UserRef>,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewInput {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub data_source_id: Option<String>,
    #[serde(default)]
    pub r#type: ViewType,
    #[serde(default)]
    pub filter: Option<Filter>,
    #[serde(default)]
    pub sorts: Option<Vec<Sort>>,
    #[serde(default)]
    pub configuration: Option<ViewConfig>,
}

impl Default for ViewInput {
    fn default() -> Self {
        Self {
            name: String::new(),
            data_source_id: None,
            r#type: ViewType::Table,
            filter: None,
            sorts: None,
            configuration: Some(ViewConfig::Table),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ViewQueryInput {
    #[serde(default)]
    pub filter: Option<Filter>,
    #[serde(default)]
    pub sorts: Option<Vec<Sort>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewQueryResult {
    pub view_id: String,
    pub r#type: ViewType,
    pub groups: Vec<ViewGroup>,
    pub pages: Vec<Page>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

impl ViewQueryResult {
    pub fn new(
        view: &View,
        groups: Vec<ViewGroup>,
        pages: Vec<Page>,
        has_more: bool,
        next_cursor: Option<String>,
    ) -> Self {
        Self {
            view_id: view.id.clone(),
            r#type: view.r#type,
            groups,
            pages,
            has_more,
            next_cursor,
        }
    }
}

pub(super) fn matches_filter(data_source: &DataSource, row: &Page, filter: &Filter) -> bool {
    match filter {
        Filter::And { filters } => filters
            .iter()
            .all(|filter| matches_filter(data_source, row, filter)),
        Filter::Or { filters } => filters
            .iter()
            .any(|filter| matches_filter(data_source, row, filter)),
        Filter::Equals { property, value } => property_value(data_source, row, property)
            .is_some_and(|actual| value_as_string(actual).eq_ignore_ascii_case(value)),
        Filter::NotEquals { property, value } => property_value(data_source, row, property)
            .is_none_or(|actual| !value_as_string(actual).eq_ignore_ascii_case(value)),
        Filter::Contains { property, value } => property_value(data_source, row, property)
            .is_some_and(|actual| {
                value_as_string(actual)
                    .to_lowercase()
                    .contains(&value.to_lowercase())
            }),
        Filter::IsEmpty { property } => {
            property_value(data_source, row, property).is_none_or(value_is_empty)
        }
        Filter::IsNotEmpty { property } => {
            property_value(data_source, row, property).is_some_and(|actual| !value_is_empty(actual))
        }
        Filter::GreaterThan { property, value } => property_value(data_source, row, property)
            .and_then(value_as_number)
            .is_some_and(|number| number > *value),
        Filter::LessThan { property, value } => property_value(data_source, row, property)
            .and_then(value_as_number)
            .is_some_and(|number| number < *value),
        Filter::Checked { property, value } => property_value(data_source, row, property)
            .is_some_and(
                |actual| matches!(actual, PropertyValue::Checkbox(checked) if checked == value),
            ),
    }
}

pub(super) fn compare_pages(
    data_source: &DataSource,
    a: &Page,
    b: &Page,
    sorts: &[Sort],
) -> Ordering {
    for sort in sorts {
        let left = property_value(data_source, a, &sort.property);
        let right = property_value(data_source, b, &sort.property);
        let ordering = match (left, right) {
            (Some(left), Some(right)) => match (value_as_number(left), value_as_number(right)) {
                (Some(left), Some(right)) => left.cmp(&right),
                _ => value_as_string(left).cmp(&value_as_string(right)),
            },
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        };
        let ordering = match sort.direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    Ordering::Equal
}
