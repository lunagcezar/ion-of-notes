//! Central layout metrics for the desktop UI.
//!
//! Every value is a logical point. Spacing, sizes, padding, radii and font
//! sizes use even numbers so layout arithmetic stays predictable, with two
//! documented exceptions: the 13 px `FONT_SMALL` label token and the 1 px
//! `STROKE_HAIRLINE`. Colors live in `palette.rs`; icon artwork coordinates
//! stay with `icons.rs`. This module contains only constants: no functions,
//! types or state.

// ---- Base scale -----------------------------------------------------------

pub(crate) const SPACE_2: f32 = 2.0;
pub(crate) const SPACE_4: f32 = 4.0;
pub(crate) const SPACE_6: f32 = 6.0;
pub(crate) const SPACE_8: f32 = 8.0;
pub(crate) const SPACE_10: f32 = 10.0;
pub(crate) const SPACE_12: f32 = 12.0;
pub(crate) const SPACE_14: f32 = 14.0;
pub(crate) const SPACE_16: f32 = 16.0;
pub(crate) const SPACE_18: f32 = 18.0;
pub(crate) const SPACE_20: f32 = 20.0;
pub(crate) const SPACE_22: f32 = 22.0;
pub(crate) const SPACE_24: f32 = 24.0;
pub(crate) const SPACE_26: f32 = 26.0;
pub(crate) const SPACE_28: f32 = 28.0;
pub(crate) const SPACE_30: f32 = 30.0;
pub(crate) const SPACE_32: f32 = 32.0;
pub(crate) const SPACE_34: f32 = 34.0;
pub(crate) const SPACE_36: f32 = 36.0;
pub(crate) const SPACE_38: f32 = 38.0;
pub(crate) const SPACE_40: f32 = 40.0;
pub(crate) const SPACE_42: f32 = 42.0;
pub(crate) const SPACE_44: f32 = 44.0;
pub(crate) const SPACE_46: f32 = 46.0;
pub(crate) const SPACE_48: f32 = 48.0;
pub(crate) const SPACE_52: f32 = 52.0;
pub(crate) const SPACE_60: f32 = 60.0;
pub(crate) const SPACE_64: f32 = 64.0;
pub(crate) const SPACE_70: f32 = 70.0;
pub(crate) const SPACE_72: f32 = 72.0;
pub(crate) const SPACE_76: f32 = 76.0;
pub(crate) const SPACE_80: f32 = 80.0;
pub(crate) const SPACE_88: f32 = 88.0;
pub(crate) const SPACE_96: f32 = 96.0;
pub(crate) const SPACE_120: f32 = 120.0;

// ---- Shell / window chrome ------------------------------------------------

pub(crate) const TITLE_BAR_HEIGHT: f32 = SPACE_36;
pub(crate) const SIDEBAR_WIDTH: f32 = 266.0;
pub(crate) const APP_BAR_COMPACT_WIDTH: f32 = 92.0;
pub(crate) const TITLE_BUTTON_SIZE: f32 = SPACE_26;
pub(crate) const WINDOW_CONTROL_WIDTH: f32 = SPACE_38;
pub(crate) const TAB_WIDTH: f32 = 190.0;
pub(crate) const TAB_HEIGHT: f32 = SPACE_28;
pub(crate) const TAB_RADIUS: u8 = 6;
pub(crate) const TAB_ICON_INSET: f32 = SPACE_10;
pub(crate) const TAB_ICON_Y: f32 = SPACE_6;
pub(crate) const TAB_LABEL_INSET: f32 = SPACE_34;
pub(crate) const TAB_CLOSE_SIZE: f32 = SPACE_18;
pub(crate) const TAB_CLOSE_INSET: f32 = SPACE_14;
pub(crate) const TAB_GAP: f32 = SPACE_6;
pub(crate) const BADGE_SIZE: f32 = SPACE_12;
pub(crate) const BADGE_OFFSET: f32 = SPACE_6;
pub(crate) const BADGE_RADIUS: f32 = SPACE_6;
pub(crate) const SHOW_EXTRAS_MIN_WIDTH: f32 = 150.0;

// ---- Sidebar and settings rail -------------------------------------------

/// Content inset shared by the main sidebar and the settings navigation rail.
pub(crate) const SIDEBAR_CONTENT_PADDING: f32 = SPACE_16;
/// Height shared by sidebar rows and settings navigation rows.
pub(crate) const ROW_HEIGHT: f32 = SPACE_30;
/// Corner radius shared by sidebar rows and settings navigation rows.
pub(crate) const ROW_RADIUS: u8 = 6;
/// Glyph box shared by sidebar and settings navigation rows.
pub(crate) const ICON_SIZE: f32 = SPACE_16;
/// Gap between sidebar sections and between settings navigation groups.
pub(crate) const SECTION_GAP: f32 = SPACE_18;
/// Height of a sidebar / settings section label.
pub(crate) const SECTION_LABEL_HEIGHT: f32 = SPACE_24;
/// Glyph inset inside a sidebar row.
pub(crate) const ROW_ICON_INSET: f32 = SPACE_10;
/// Label inset inside a sidebar row.
pub(crate) const ROW_LABEL_INSET: f32 = SPACE_36;
/// Glyph inset inside a settings navigation row.
pub(crate) const SETTINGS_NAV_ICON_INSET: f32 = SPACE_12;
/// Label inset inside a settings navigation row.
pub(crate) const SETTINGS_NAV_LABEL_INSET: f32 = SPACE_38;
/// Gap between the settings rail search and the navigation list.
pub(crate) const SETTINGS_NAV_GAP: f32 = SPACE_14;
/// Height of the sidebar search field.
pub(crate) const SIDEBAR_SEARCH_HEIGHT: f32 = SPACE_34;
/// Height of the settings rail search field.
pub(crate) const SETTINGS_SEARCH_HEIGHT: f32 = ROW_HEIGHT;
/// Height of the sidebar account row.
pub(crate) const SIDEBAR_ACCOUNT_HEIGHT: f32 = SPACE_40;
/// Height of the invite card.
pub(crate) const SIDEBAR_INVITE_HEIGHT: f32 = SPACE_76;
/// Height of the customize "Done" button.
pub(crate) const SIDEBAR_DONE_HEIGHT: f32 = SPACE_32;
/// Height of the customize-sidebar row.
pub(crate) const SIDEBAR_CUSTOMIZE_HEIGHT: f32 = SPACE_28;
/// Height of a shortcut row inside a custom sidebar section.
pub(crate) const SIDEBAR_SHORTCUT_HEIGHT: f32 = SPACE_28;
/// Width reserved on the right of the account row for Help and the grid menu.
pub(crate) const SIDEBAR_ACCOUNT_ACTIONS_WIDTH: f32 = SPACE_52;
/// Minimum height of the scrolling sidebar list.
pub(crate) const SIDEBAR_MIN_SCROLL_HEIGHT: f32 = SPACE_120;
/// Space reserved below the scrolling sidebar list for the fixed bottom group.
pub(crate) const SIDEBAR_BOTTOM_RESERVE: f32 = 176.0;

// ---- Settings dialog ------------------------------------------------------

pub(crate) const SETTINGS_RAIL_WIDTH: f32 = 300.0;
pub(crate) const SETTINGS_PADDING: f32 = SPACE_20;
pub(crate) const SETTINGS_RAIL_PADDING_Y: f32 = SIDEBAR_CONTENT_PADDING;
pub(crate) const SETTINGS_CONTENT_PADDING: f32 = SPACE_24;
pub(crate) const SETTINGS_SEARCH_WIDTH: f32 = SETTINGS_RAIL_WIDTH - 2.0 * SIDEBAR_CONTENT_PADDING;
pub(crate) const SETTINGS_SEARCH_ICON_INSET: f32 = SPACE_8;
pub(crate) const SETTINGS_SEARCH_TEXT_INSET: f32 = SPACE_28;
pub(crate) const SETTINGS_CONTENT_MIN_WIDTH: f32 = 320.0;
pub(crate) const SETTINGS_CLOSE_SIZE: f32 = SPACE_28;
pub(crate) const SETTINGS_CLOSE_INSET: f32 = SPACE_42;
pub(crate) const SETTINGS_MIN_WIDTH: f32 = 720.0;
pub(crate) const SETTINGS_MAX_WIDTH: f32 = 1200.0;
pub(crate) const SETTINGS_MIN_HEIGHT: f32 = 420.0;
pub(crate) const SETTINGS_MAX_HEIGHT: f32 = 720.0;
pub(crate) const DIALOG_RADIUS: u8 = 10;
pub(crate) const DIALOG_INSET: f32 = 40.0;
pub(crate) const CHIP_WIDTH: f32 = 150.0;
pub(crate) const LANGUAGE_CHIP_WIDTH: f32 = 170.0;
pub(crate) const CHIP_HEIGHT: f32 = SPACE_26;

// ---- Content page and breadcrumbs ----------------------------------------

pub(crate) const BREADCRUMB_HEIGHT: f32 = SPACE_28;
pub(crate) const BREADCRUMB_GAP: f32 = SPACE_6;
pub(crate) const BREADCRUMB_PADDING: f32 = SPACE_16;
pub(crate) const BREADCRUMB_ICON_SIZE: f32 = SPACE_16;
pub(crate) const BREADCRUMB_ICON_INSET: f32 = SPACE_2;
pub(crate) const BREADCRUMB_LABEL_INSET: f32 = SPACE_24;
pub(crate) const BREADCRUMB_ACTION_SIZE: f32 = SPACE_26;
pub(crate) const BREADCRUMB_ACTION_HEIGHT: f32 = SPACE_24;
pub(crate) const CONTENT_SIDE_MARGIN: f32 = SPACE_96;
pub(crate) const CONTENT_COLUMN_MIN: f32 = 360.0;
pub(crate) const CONTENT_COLUMN_MAX: f32 = 708.0;
pub(crate) const CONTENT_GUTTER: f32 = SPACE_48;
pub(crate) const CONTENT_TOP_SPACE: f32 = SPACE_88;
pub(crate) const CONTENT_BOTTOM_SPACE: f32 = SPACE_80;
pub(crate) const PAGE_ICON_SIZE: f32 = SPACE_72;
pub(crate) const PAGE_ICON_RADIUS: u8 = 10;
pub(crate) const PAGE_TITLE_GAP: f32 = SPACE_30;
pub(crate) const PAGE_TITLE_BOTTOM: f32 = SPACE_16;
pub(crate) const PAGE_TOC_GAP: f32 = SPACE_20;
pub(crate) const PAGE_DISCUSSION_GAP: f32 = SPACE_26;
pub(crate) const FLOATING_CONTROL_INSET: f32 = SPACE_44;
pub(crate) const FLOATING_CONTROL_SIZE: f32 = SPACE_24;
pub(crate) const FLOATING_LINES_TOP: f32 = 280.0;
pub(crate) const FLOATING_LINES_SIZE: f32 = SPACE_20;

// ---- Blocks ---------------------------------------------------------------

pub(crate) const BLOCK_ROW_HEIGHT: f32 = SPACE_30;
pub(crate) const BLOCK_CONTROL_WIDTH: f32 = SPACE_22;
pub(crate) const BLOCK_CONTROL_HEIGHT: f32 = SPACE_22;
pub(crate) const BLOCK_CONTROL_GAP: f32 = SPACE_4;
pub(crate) const BLOCK_GAP: f32 = SPACE_14;
pub(crate) const BLOCK_HEADING_GAP: f32 = SPACE_22;
pub(crate) const BLOCK_BODY_SIZE: f32 = SPACE_16;
pub(crate) const BLOCK_SMALL_SIZE: f32 = SPACE_14;
pub(crate) const BLOCK_MONO_SIZE: f32 = SPACE_14;
pub(crate) const BLOCK_CALLOUT_PADDING: f32 = SPACE_12;
pub(crate) const BLOCK_CODE_PADDING: f32 = SPACE_12;
pub(crate) const BLOCK_BULLET_SIZE: f32 = SPACE_6;
pub(crate) const BLOCK_TODO_SIZE: f32 = SPACE_18;
pub(crate) const BLOCK_DIVIDER_HEIGHT: f32 = SPACE_14;
pub(crate) const BLOCK_DRAG_OVER: f32 = SPACE_10;
pub(crate) const DRAG_GHOST_MIN_WIDTH: f32 = 220.0;
pub(crate) const DRAG_GHOST_MAX_WIDTH: f32 = 420.0;

// ---- Popovers, menus and shared rows -------------------------------------

pub(crate) const POPOVER_RADIUS: u8 = 8;
pub(crate) const POPOVER_PADDING: f32 = SPACE_6;
pub(crate) const POPOVER_GAP: f32 = 0.0;
pub(crate) const APP_MENU_WIDTH: f32 = 430.0;
pub(crate) const APP_MENU_LABEL_WIDTH: f32 = 150.0;
pub(crate) const APP_MENU_HINT_WIDTH: f32 = 250.0;
pub(crate) const COMPOSE_MENU_LEFT: f32 = 150.0;
pub(crate) const COMPOSE_MENU_TOP: f32 = 42.0;
pub(crate) const COMPOSE_MENU_WIDTH: f32 = 230.0;
pub(crate) const POPOVER_WORKSPACE_WIDTH: f32 = 250.0;
pub(crate) const POPOVER_GRID_WIDTH: f32 = 180.0;
pub(crate) const POPOVER_SIDEBAR_WIDTH: f32 = 260.0;
pub(crate) const SIDEBAR_MENU_SHORTCUT_Y: f32 = 300.0;
pub(crate) const SIDEBAR_MENU_SECTION_Y: f32 = 420.0;
pub(crate) const SIDEBAR_MENU_ROW_Y: f32 = 360.0;
pub(crate) const SLASH_MENU_MAX_HEIGHT: f32 = 330.0;
pub(crate) const SWITCHER_WIDTH_WIDE: f32 = 1000.0;
pub(crate) const SWITCHER_WIDTH: f32 = 600.0;
pub(crate) const SWITCHER_MAX_HEIGHT: f32 = 690.0;
pub(crate) const SWITCHER_MIN_HEIGHT: f32 = 360.0;
pub(crate) const SWITCHER_TOP: f32 = 110.0;
pub(crate) const SWITCHER_MARGIN: f32 = 40.0;
pub(crate) const SWITCHER_BODY_OFFSET: f32 = 92.0;
pub(crate) const SWITCHER_BODY_MIN: f32 = 200.0;
pub(crate) const SWITCHER_LIST_PERCENT: f32 = 56.0;
pub(crate) const SWITCHER_HINT_WIDTH: f32 = 150.0;
pub(crate) const SWITCHER_ROW_HEIGHT: f32 = 34.0;
pub(crate) const MENU_ROW_HEIGHT: f32 = ROW_HEIGHT;
pub(crate) const MENU_ROW_RADIUS: u8 = 6;
pub(crate) const MENU_TEXT_SIZE: f32 = SPACE_14;
pub(crate) const MENU_HINT_SIZE: f32 = SPACE_12;
pub(crate) const ICON_RADIUS: u8 = 4;
pub(crate) const RADIUS_PILL: u8 = 14;

// ---- Font sizes -----------------------------------------------------------

pub(crate) const ICON_PICKER_COLUMNS: usize = 9;
pub(crate) const ICON_PICKER_ROW_HEIGHT: f32 = SPACE_32;

pub(crate) const FONT_MICRO: f32 = SPACE_10;
pub(crate) const FONT_CAPTION: f32 = SPACE_12;
/// 13 px is a common compact label size. Font sizes are not layout
/// arithmetic, so this is the documented odd font token alongside the
/// hairline stroke.
pub(crate) const FONT_SMALL: f32 = 13.0;
pub(crate) const FONT_BODY: f32 = SPACE_14;
pub(crate) const FONT_ROW: f32 = FONT_SMALL;
pub(crate) const FONT_SUBTITLE: f32 = SPACE_18;
pub(crate) const FONT_TITLE: f32 = SPACE_28;
pub(crate) const FONT_PAGE_TITLE: f32 = SPACE_44;
pub(crate) const FONT_CLOSE: f32 = SPACE_18;

// ---- Strokes --------------------------------------------------------------

/// Hairline strokes stay 1 px so borders remain crisp at 1x. This and
/// `FONT_SMALL` are the documented non-even tokens.
pub(crate) const STROKE_HAIRLINE: f32 = 1.0;
pub(crate) const STROKE_EMPHASIS: f32 = SPACE_2;
