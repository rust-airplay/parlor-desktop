# GPUI Kit 0.9.0 Cheat Sheet (Windows / MSVC)
Every signature below was read from the cloned source at `D:\tmp\gpui-kit` (main, commit `6fdd3e7`): `crates/kit`, `crates/component`, `crates/base`, `examples/`, `skills/`, `themes/`. GPUI itself is the pinned snapshot `gpui-pre` 0.3.11; paths starting `gpui-pre-0.3.11/` refer to that snapshot, every other path is repo-relative. Nothing comes from memory or another GPUI version.

## 1. Cargo setup
```toml
[package]
name = "parlor-desktop"
version = "0.1.0"
edition = "2024"          # workspace edition is 2024 (Cargo.toml, [workspace.package])

[dependencies]
gpui-kit = "0.9.0"        # default features = ["component", "assets"]
# gpui-kit = { version = "0.9.0", features = ["gpui-fast"] }   # optional backend swap

[dev-dependencies]
gpui-kit = { version = "0.9.0", features = ["test-support"] }  # UI tests
```

One dependency carries GPUI, GPUI Base, the styled components and the default icons (`crates/kit/Cargo.toml`, `crates/kit/src/lib.rs`). `component` (default) enables `gpui_kit::component`; `assets` (default) provides `gpui_kit::assets::Assets`; `gpui-fast` swaps the backend (`gpui-fast`, `gpui-fast-platform`, `gpui-fast-web`) across every enabled layer with no import change, no `[patch]` and no crate alias (features are additive, so never mix Kit Fast types with a direct upstream GPUI dependency); `test-support` adds the GPUI test harness and `gpui_kit::test`.

Windows (`website/docs/installation.md`): Windows 10+, VS 2022 Build Tools with Desktop development with C++, MSVC + Windows SDK, CMake on `PATH`, an `msvc` Rust toolchain, Rust 1.92+. Do not override `gpui-pre`: Kit pins `=0.3.11`.

Register assets before `run` (`examples/window_title/src/main.rs`): `gpui_kit::application().with_assets(gpui_kit::assets::Assets)`. Without them icon paths do not resolve (`examples/hello_world` therefore shows text and a label-only Button).

## 2. Application bootstrap
```rust
use gpui_kit::*;

struct AppView;

impl Render for AppView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("Parlor")
    }
}

fn main() {
    gpui_kit::application().with_assets(gpui_kit::assets::Assets).run(|cx| {
        gpui_kit::init(cx); // MUST be first: Base + Component, theme registry, keys

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1100.), px(760.)), cx)),
            window_min_size: Some(size(px(720.), px(480.))),
            titlebar: Some(TitlebarOptions { title: Some("Parlor".into()), ..Default::default() }),
            ..Default::default()
        };

        gpui_kit::open_window(options, cx, |window, cx| {
            window.set_window_title("Parlor");
            cx.new(|cx| AppView)
        })
        .expect("failed to open window");
    });
}
```

- `open_window(options, cx, build) -> Result<(AnyWindowHandle, Entity<V>)>` (`crates/kit/src/lib.rs`) returns the content entity, not the `Root`.
- Custom title bar: render `TitleBar::new()` in the view and open with `TitleBar::window_options()` (`examples/window_title/src/main.rs`). `WindowOptions` (`gpui-pre-0.3.11/src/platform.rs:2602`): `window_bounds: Option<WindowBounds>`, `window_min_size: Option<Size<Pixels>>`, `titlebar: Option<TitlebarOptions>`, `app_id: Option<String>`, `is_resizable`, `kind`, `icon`. `TitlebarOptions` (`:2825`): `title`, `appears_transparent`, `traffic_light_position`. `size(px(w), px(h))` and `WindowBounds::centered(size, cx)` are GPUI (`examples/sidebar/src/main.rs`).
- Quit on last window close is the Windows default: `QuitMode::Default` resolves to `LastWindowClosed` everywhere except macOS (`gpui-pre-0.3.11/src/app.rs:526`). For a menu command bind an action and call `cx.quit()` (`gpui-pre-0.3.11/src/app.rs:1343`, used in `examples/system_monitor/src/main.rs`).
- App icon: not supported on Windows. `WindowOptions::icon` is X11 only (`gpui-pre-0.3.11/src/platform.rs:2679`); no `set_app_icon`/`with_icon` exists. One `Root` per window: `open_window` mounts it and the old `Root::render_*_layer` methods were removed in 0.7.0 (`skills/gpui-kit/references/usage.md`).

## 3. Theme
`Theme` is a global (`crates/component/src/theme/mod.rs:150`), read through `ActiveTheme`:

```rust
use gpui_kit::component::ActiveTheme as _;

let theme = cx.theme(); // &Theme, Deref<Target = ThemeColor>
div().bg(theme.background).text_color(theme.foreground)
    .border_1().border_color(theme.border).rounded(theme.radius)
```

Semantic tokens are public `ThemeColor` fields (`crates/component/src/theme/theme_color.rs:61`): `background`, `foreground`, `border`, `primary`, `primary_foreground`, `muted`, `muted_foreground`, `secondary`, `secondary_foreground`, `accent`, `danger`, `danger_foreground` (there is no `destructive` field), `popover`, `input`, `ring`, `sidebar`, `sidebar_foreground`, `success`, `warning`, `info`. On `Theme`: `radius`, `radius_lg`, `font_size` (`theme/mod.rs:150`). `ThemeMode` has exactly `Light` and `Dark` (`theme/mod.rs:814`); 0.9.0 has no `System` variant.

```rust
use gpui_kit::component::{Theme, ThemeMode, ThemeRegistry};

// switch the application (works from &mut App and from Context<T>, which Derefs to App)
Theme::change(ThemeMode::Dark, None, cx);
// edit tokens; update keeps colors, gradient tokens and the Base projection in step
Theme::update(cx, |theme| theme.radius = px(8.));
// load a registered named theme
if let Some(config) = ThemeRegistry::global(cx).themes().get(&name).cloned() {
    Theme::update(cx, |theme| theme.apply_config(&config));
}
cx.refresh_windows();
```

Do not stop after `Theme::global_mut(cx).radius = ...`: it leaves `tokens` and the Base projection stale, so scrollbars and resize handles keep the old style. `Theme::update` and `Theme::change` are the write paths (`theme/mod.rs:295`, `:396`, `:479`).

System appearance follows the window, because the mode cannot express it:

```rust
cx.observe_window_appearance(window, |_this, window, cx| {
    Theme::sync_system_appearance(Some(window), cx);
})
.detach();
```

`Theme::sync_system_appearance` reads `window.appearance()` and calls `Theme::change` (`theme/mod.rs:361`); `Context::observe_window_appearance` is `gpui-pre-0.3.11/src/app/context.rs:441`; `Window::appearance` is `gpui-pre-0.3.11/src/window.rs:3144`.

Built-in themes: exactly two are compiled in, `Default Light` and `Default Dark` (`crates/component/src/theme/default-theme.json` via `theme/registry.rs`). The JSON families in `themes/` (Adventure, Alduin, Asciinema, Aurora, Ayu, Catppuccin, Everforest, Fahrenheit, Flexoki, Gruvbox, Harper, Hybrid, Jellybeans, Kibble, macOS Classic, Mellifluous, Molokai, Solarized, Spaceduck, Tokyo Night, Twilight) load only at runtime:

```rust
ThemeRegistry::watch_dir(std::path::PathBuf::from("./themes"), cx, |cx| cx.refresh_windows())?;
// or without a watcher: ThemeRegistry::global_mut(cx).load_themes_from_str(&json)?;
```

Persist the choice yourself; the registry has no storage. The pattern is `crates/story/src/themes.rs`: `cx.observe_global::<Theme>(|cx| { let json = serde_json::json!({ "theme": cx.theme().theme_name().to_string(), "mode": cx.theme().mode.name() }); let _ = std::fs::write(config_path(), json.to_string()); }).detach();`, restoring at startup through `Theme::change` / `apply_config`. `theme_name()` is `theme/mod.rs:347`; `ThemeMode::name()` is `theme/mod.rs:832`.

## 4. Components
Paths are under `gpui_kit::component::`. Builder traits must be imported: `Sizable` (`xsmall`, `small`, `large`, `with_size`), `Disableable` (`disabled`), `Selectable` (`selected`), `ButtonVariants` (`primary`, `danger`, `ghost`, ...), all re-exported by `crates/component/src/styled.rs`.

| Component | Import | State | Constructor / key builders | Event |
| --- | --- | --- | --- | --- |
| Button | `button::Button` | stateless | `Button::new(id)`, `.label(..)`, `.icon(IconName::Plus)`, `.primary() .danger() .outline() .ghost() .link()`, `.xsmall() .small() .large()`, `.selected(bool)`, `.loading(bool)`, `.tooltip(..)` | `.on_click(Fn(&ClickEvent, &mut Window, &mut App))` |
| Toggle | `button::Toggle` | stateless | `Toggle::new(id)`, `.checked(bool)`, `.label(..)`, `.tooltip(..)` | `.on_click(Fn(&bool, ..))` |
| Switch | `switch::Switch` | controlled | `Switch::new(id)`, `.checked(bool)`, `.label(impl Into<Text>)`, `.color(Hsla)` | `.on_change(Fn(&bool, &mut Window, &mut App))`; `on_click` alias |
| Checkbox | `checkbox::Checkbox` | controlled | `Checkbox::new(id)`, `.checked(bool)`, `.label(..)` | `.on_change(Fn(&bool, ..))` |
| Select | `select::{Select, SelectState}` | `Entity<SelectState<D>>` | `SelectState::new(items, None, window, cx)`, `Select::new(&state).placeholder(..).cleanable(bool)` | `SelectEvent::Confirm(Option<Value>)`; read `.selected_value()` |
| Slider | `slider::{Slider, SliderState}` | `Entity<SliderState>` | `SliderState::new().min(0.).max(100.).step(1.).default_value(50.)`, `Slider::new(&state).horizontal()` | `SliderEvent::Change/Release(SliderValue)` |
| Text input | `input::{Input, InputState, InputEvent}` | `Entity<InputState>` | `InputState::new(window, cx).placeholder("Search")`, `Input::new(&state).id("name").cleanable(true).prefix(el)` | `InputEvent::Change`, `PressEnter { secondary, shift }`, `Focus`, `Blur` |
| TextArea | `input::{Textarea, TextareaState}` | `Entity<TextareaState>` | `TextareaState::new(window, cx)`, `Textarea::new(&state).h(px(120.))` | same `InputEvent` |
| NumberInput | `input::{NumberInput, NumberInputEvent}` | `Entity<InputState>` | `NumberInput::new(&state)`, `InputState::new(window, cx).step_by(..)` | `NumberInputEvent::Step(StepAction)` |
| Label | `label::Label` | stateless | `Label::new(impl Into<SharedString>)`, `.secondary(..)`, `.masked(bool)` | none |
| Icon | `Icon`, `IconName` | stateless | `Icon::new(IconName::Check).small().text_color(..)` | none |
| Tooltip | `tooltip::Tooltip` | stateless | `.tooltip("text")` on Button/Switch/Checkbox/Radio/Input/Clipboard; `Tooltip::new(text).build(window, cx)` -> `AnyView` | none |
| Dialog | `dialog::Dialog` + `WindowExt` | window-owned | `window.open_dialog(cx, \|dialog, _, _\| dialog.title(..).child(..).footer(..))` | `.on_ok(..)`, `.on_cancel(..)`, `.on_close(..)` |
| AlertDialog | `dialog::AlertDialog` + `WindowExt` | window-owned | `window.open_alert_dialog(cx, \|alert, _, _\| alert.title(..).description(..).confirm().on_ok(\|_, _, _\| true))` | `on_ok` returns `bool` (keep open) |
| Notification | `notification::Notification` + `WindowExt` | window-owned | `window.push_notification(Notification::success("Saved"), cx)`; `Notification::info/warning/error(msg)` | `.on_click(..)`, `.on_close(..)`, `.autohide(bool)` |
| Tabs | `tab::{TabBar, Tab}` | controlled index | `TabBar::new("tabs").selected_index(ix).child(Tab::new().label("General"))` | `.on_click(Fn(&usize, &mut Window, &mut App))` |
| Sidebar | `sidebar::{Sidebar, SidebarMenu, SidebarMenuItem, SidebarGroup, SidebarHeader, SidebarFooter, SidebarToggleButton, SidebarCollapsible}` | controlled | `Sidebar::new(id).collapsible(..).collapsed(bool).header(..).child(SidebarGroup::new("Nav").child(menu)).footer(..)` | `SidebarMenuItem::on_click` |
| List | `list::{List, ListState, ListDelegate, ListItem, ListEvent}` | `Entity<ListState<D>>` | `ListState::new(delegate, window, cx)`, `List::new(&state)` | `ListEvent::Select/Confirm(IndexPath)`, `Cancel` |
| VirtualList | `{v_virtual_list, h_virtual_list}` + `VirtualListScrollHandle` | scroll handle | `v_virtual_list(id, count, sizes, render_item)` | n/a |
| Scrollable | `scroll::ScrollableElement` (trait) | none | `.id("page")` then `.overflow_y_scrollbar()` / `.overflow_x_scrollbar()` | none |
| DataTable | `table::{DataTable, TableState, TableDelegate, Column, ColumnSort}` | `Entity<TableState<D>>` | `TableState::new(delegate, window, cx)`, `DataTable::new(&state).stripe(true).bordered(true).small()` | `TableDelegate` callbacks, `TableEvent` |
| Simple table | `table::{Table, TableHeader, TableRow, TableHead, TableCell}` | stateless | `Table::new().child(TableHeader::new().child(TableRow::new().child(TableHead::new().child("ID"))))` | none |
| Progress | `progress::{Progress, ProgressCircle}` | stateless | `Progress::new("id").value(42.).loading(bool)` | none |
| Spinner | `spinner::Spinner` | stateless | `Spinner::new().small()` | none |
| Divider | `separator::Separator` | stateless | `Separator::horizontal()`, `::vertical()`, `::horizontal_dashed()`, `.label("or")` | none |
| Badge | `badge::Badge` | stateless | `Badge::new().count(3)`, `.dot()`, `.max(99)`, `.icon(..)`, `.color(..)` | none |

Sources, relative to `crates/component/src/` (paths that start with `crates/` are repo-relative): `button/button.rs:243,459`; `button/toggle.rs:50`; `switch.rs:35,56`; `checkbox.rs:39,96`; `select.rs:154,679`; `slider.rs:97` + `crates/base/src/slider.rs:196,452`; `input/input.rs:234`; `input/textarea.rs:70`; `input/number_input.rs:36`; `label.rs:65`; `icon.rs:136`; `tooltip.rs:48`; `window_ext.rs:75,101,118`; `dialog/dialog.rs:364`; `notification.rs:174,203`; `tab/tab_bar.rs:68,173,187`; `sidebar/mod.rs:238`; `list/list.rs:104,751`; `scroll/scrollable.rs:16`; `table/data_table.rs:112`; `table/state.rs:311`; `progress/progress.rs:26`; `spinner.rs:20`; `separator.rs:28`; `badge.rs:43`.

```rust
use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
    switch::Switch,
};
use gpui_kit::{AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _, Subscription, Window, div};

pub struct SettingsView { name: Entity<InputState>, enabled: bool, _subs: Vec<Subscription> }

impl SettingsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Display name"));
        let sub = cx.subscribe_in(&name, window, |_this, state, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) { let _value = state.read(cx).value(); }
        });
        Self { name, enabled: false, _subs: vec![sub] }
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().v_flex().size_full().gap_3().p_4()
            .bg(cx.theme().background).text_color(cx.theme().foreground)
            .child(Input::new(&self.name).id("name"))
            .child(Switch::new("enabled").checked(self.enabled).label("Enable sync")
                .on_change(cx.listener(|this, next, _, cx| { this.enabled = *next; cx.notify(); })))
            .child(Button::new("about").label("About").icon(IconName::Info)
                .on_click(|_, window, cx| {
                    window.open_dialog(cx, |dialog, _, _| {
                        dialog.title("About Parlor").child("GPUI Kit 0.9.0")
                    });
                }))
    }
}
```

`Slider` and `NumberInput` follow the same shape: `cx.new` the state, render the element with `&state`, subscribe for events (`impl EventEmitter<SliderEvent> for SliderState`, `crates/base/src/slider.rs:888`; `NumberInputEvent`, `crates/base/src/number_input.rs:57`).

## 5. Layout and styling
```rust
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _; // .when(..), .when_some(..)

v_flex()                       // div().flex().flex_col(); children stretch on the cross axis
    .size_full()               // w_full + h_full
    .gap_3()                   // rem scale: gap_1 = 4px, gap_2 = 8px, gap_4 = 16px
    .p_4()                     // p_/px_/py_/pt_/pl_/pr_; m_/mx_/my_ likewise
    .child(h_flex().gap_2().items_center().child("Label").child("Value"))
    .child(
        div().flex_1()         // fill the remaining main-axis space
            .min_w_0()         // REQUIRED on a shrinkable flex child with wide content
            .max_w_64().min_h(px(120.))  // rem caps; max_w / min_w / min_h take a Length
            .rounded(cx.theme().radius).border_1().border_color(cx.theme().border)
            .shadow_md().text_sm().font_weight(FontWeight::SEMIBOLD)
            .text_color(cx.theme().muted_foreground)
            .overflow_hidden().child("Region"),
    )
    .child(Separator::horizontal())
```

Rules from `skills/gpui-kit/references/coding-guides.md` (Layout, measurement, and scrolling):

- `h_flex()` centers children on the cross axis; `v_flex()` lets them stretch. A full-height column in a row needs `h_full()`, or the row needs `items_stretch()`; otherwise the column is centered and its header clips.
- `flex_1()` fills remaining main-axis space; `w_full()` is 100 percent of the parent width and does not take part in flex distribution. Use `flex_1()` for the growing pane, `w(px(240.))` for a fixed sidebar.
- A flexible child will not shrink below its content unless its overflow is not visible: put `min_w_0()` (row) or `min_h_0()` (column) on it. `Scrollable` handles its own wrapper, but a plain `div` between it and the flex container still needs the minimum released.
- One owner per scrollable region: `.overflow_y_scrollbar()` goes on the element that owns the full panel; padding goes inside that owner, not on a wrapper.
- Prefer rem helpers (`p_2`, `gap_3`, `text_sm`, `w_64`, `size_4`) over `px(...)`; the theme base font is the application zoom. There is no `z_index` (`skills/gpui-kit/references/gpui/layout-style.md`).

Scrollable column of settings rows (import the trait, as `examples/table_in_scrollable/src/main.rs` does):

```rust
use gpui_kit::component::{scroll::ScrollableElement as _, separator::Separator, *};

div().id("settings-scroll").size_full()
    .overflow_y_scrollbar()  // the scroll owner; the scrollbar resolves against this edge
    .child(
        v_flex().p_4().gap_3()
            .child(h_flex().justify_between().child("Theme").child("Dark"))
            .child(Separator::horizontal())
            .child(h_flex().justify_between().child("Language").child("English"))
            .child(Separator::horizontal())
            .child(h_flex().justify_between().child("Updates").child("Automatic")),
    )
```

## 6. State and events
`RenderOnce` for value-like elements; `Entity<T>` for behavior that spans frames (`skills/gpui-kit/references/coding-guides.md`, Choose the right unit).

```rust
use gpui_kit::{Context, Entity, EventEmitter, IntoElement, Render, Subscription, div};
#[derive(Clone)]
pub enum NameEvent { Submitted(String) }

pub struct NameField { value: String }
impl EventEmitter<NameEvent> for NameField {} // required before cx.emit

impl NameField {
    fn submit(&mut self, cx: &mut Context<Self>) {
        cx.emit(NameEvent::Submitted(self.value.clone()));
        cx.notify(); // schedules a re-render of this entity
    }
}

pub struct Workspace { field: Entity<NameField>, _subs: Vec<Subscription> }

impl Workspace {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let field = cx.new(|_| NameField { value: String::new() });
        let sub = cx.subscribe(&field, |_this, _emitter, event: &NameEvent, cx| {
            let NameEvent::Submitted(value) = event;
            let _ = value;
            cx.notify();
        });
        Self { field, _subs: vec![sub] }
    }
}
```

- `cx.new(|cx| T::new(..))` creates an entity; `entity.read(cx)`, `entity.update(cx, |this, cx| ..)`, `entity.downgrade()` gives a `WeakEntity`. `WeakEntity::update` returns `Result`, which is how async code detects a closed view.
- `cx.subscribe(&entity, ..)` needs no window; `cx.subscribe_in(&entity, window, ..)` gets `&mut Window`; `cx.observe(&entity, ..)` reacts to another entity calling `cx.notify()`. Each returns `Subscription`: keep it in the view or call `.detach()`, because dropping it cancels the callback.
- `cx.emit(event)` needs `impl EventEmitter<E> for T {}`. A parent view subscribes to children exactly as above; `Button::on_click` and value controls use plain callbacks, not events.
- Globals: `impl Global for MyState {}`, then `cx.set_global(state)`, `cx.global::<MyState>()`, `cx.update_global::<MyState, _>(|state, cx| ..)`, `cx.observe_global::<MyState>(|cx| ..)` (`skills/gpui-kit/references/gpui/global.md`). `Theme` and `ThemeRegistry` are globals (`impl Global for Theme`, `theme/mod.rs:240`).
- `AppContext` is a trait: `cx.new`, `cx.update_global`, `cx.observe_global` and `entity.update` come from it, so `use gpui_kit::*;` or `AppContext as _` is required. Globals hold per-app state; entities hold per-view state.

## 7. Async work
```rust
use gpui_kit::*;

fn start_polling(&mut self, cx: &mut Context<Self>) {
    cx.spawn(async move |this, cx| {   // `this` is WeakEntity<Self>
        loop {
            cx.background_executor().timer(std::time::Duration::from_millis(500)).await;
            if this.update(cx, |this, cx| { this.tick += 1; cx.notify(); }).is_err() {
                break; // the view is gone
            }
        }
    })
    .detach();
}
```

- `Context::spawn` closure shape is `AsyncFnOnce(WeakEntity<T>, &mut AsyncApp) -> R` (`gpui-pre-0.3.11/src/app/context.rs:225`). Dropping a `Task` cancels it, so store it in the view when cancellation must be controlled.
- Heavy or blocking work runs in the background and is awaited from the foreground; `background_spawn` needs a `Send` future (`gpui-pre-0.3.11/src/app/context.rs:831`): `cx.spawn(async move |this, cx| { let rows = cx.background_spawn(async move { parse(path) }).await; this.update(cx, |this, cx| { this.rows = rows; cx.notify(); }).ok(); }).detach();`
- Timers: `cx.background_executor().timer(Duration::from_millis(500)).await` (`gpui-pre-0.3.11/src/executor.rs:200`); `examples/system_monitor/src/main.rs` uses `smol::Timer::after(INTERVAL).await` in the same loop. Both are valid.
- Polling `std::sync::mpsc` from the UI thread: keep work on its own thread, drain the receiver in a foreground loop, notify once per batch:

```rust
let (tx, rx) = std::sync::mpsc::channel::<String>();
std::thread::spawn(move || { for line in produce_lines() { if tx.send(line).is_err() { break; } } });

cx.spawn(async move |this, cx| {
    loop {
        cx.background_executor().timer(std::time::Duration::from_millis(100)).await;
        let mut batch = Vec::new();
        while let Ok(line) = rx.try_recv() { batch.push(line); }
        if batch.is_empty() { continue; }
        if this.update(cx, |this, cx| { this.lines.extend(batch); cx.notify(); }).is_err() {
            break;
        }
    }
})
.detach();
```

For tokio, keep the runtime off the UI thread: run it on its own runtime (or inside `cx.background_spawn`) and forward results into the same `std::sync::mpsc` channel. Never call `entity.update` from a background thread (`skills/gpui-kit/references/gpui/async.md`).

## 8. Actions and keybindings
```rust
use gpui_kit::*;

actions!(parlor, [Quit, OpenSettings, ToggleTheme]);

// once, after gpui_kit::init
cx.bind_keys([
    KeyBinding::new("ctrl-,", OpenSettings, None),
    KeyBinding::new("ctrl-t", ToggleTheme, Some("Workspace")),
]);
cx.on_action(|_: &Quit, cx: &mut App| cx.quit());

// render: the handler region needs a key_context matching the binding context
div().key_context("Workspace").track_focus(&self.focus_handle)
     .on_action(cx.listener(Self::open_settings))
```

- `actions!` is re-exported by the Kit facade (`crates/kit/src/lib.rs`). Parameterized: `#[derive(Action, Clone, PartialEq)] #[action(namespace = parlor, no_json)] pub struct SwitchMode(pub ThemeMode);`.
- `cx.listener` adapts `fn(&mut Self, &Action, &mut Window, &mut Context<Self>)` into `Fn(&Action, &mut Window, &mut App)` (`gpui-pre-0.3.11/src/app/context.rs:241`).
- Focus: `cx.focus_handle()`; Tab stops need `cx.focus_handle().tab_stop(true)`; move focus with `window.focus(&handle, cx)`; attach with `.track_focus(&handle)`. Modal surfaces must trap focus and restore the previous target. Bind keys before `cx.set_menus`, which copies shortcuts at call time (`skills/gpui-kit/references/coding-guides.md`).

## 9. Testing
```rust
// [dev-dependencies] gpui-kit = { version = "0.9.0", features = ["test-support"] }
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{AppContext, TestAppContext};

#[gpui_kit::test]
fn saves_a_profile(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(640.), px(480.)), cx)),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| SettingsView::new(window, cx)),
        )
        .expect("open test window")
    });

    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("name", cx);
        window.input("Ada", cx);
        assert_eq!(window.find("name").value(), Some("Ada"));
    })
    .unwrap();
    cx.update(|cx| assert_eq!(view.read(cx).enabled, false));
}
```

A custom element needs `.id("status").test_support()` (`gpui_kit::test::TestSupportExt`) before `.track_focus`; then `window.find("status").label()` reads the accessibility label. Reference: `crates/kit/tests/ui.rs` and `skills/gpui-kit/references/gpui/test.md` (`find`, `try_find`, `find_all`, `click`, `press`, `input`, `scroll`, `drag`, `bounds`). Testable headless: entities, events, subscriptions, actions, async tasks, focus, layout. Not testable: window-manager behavior, native menus, OS appearance changes, pixel rendering (the `rendering` test needs Metal/WGPU).

## 10. Naming and file conventions
From `skills/gpui-kit/references/coding-guides.md` (File and naming conventions, Public API design):

- Retained behavioral model: `<Control>State` (`InputState`, `TableState`). Event: `<Control>Event`. Imperative handle: `<Control>Handle`. Pluggable owner: `<Role>Delegate`. Rendered control: noun (`Button`, `Tab`).
- Constructors use `new` or a semantic constructor (`horizontal`). Fluent builders consume and return `Self` and use domain words (`label`, `checked`, `selected`); replacement builders use `with_<field>`; in-place mutation uses `set_<field>`; boolean readers use `is_`/`has_`, other readers the field noun; callbacks use `on_<event or intent>`.
- Spell `Context` out: `cx` is GPUI, so name any other context after what it holds. Suffix ids with `Id`, zero-based indices with `ix` (never `idx`).
- `ElementId` must be stable and domain-derived: `Button::new(("delete-project", project.id))`. Never key reorderable rows by index, never generate a random id in `render`.
- No `pub` fields across the public seam: builders plus readers; public structs with public fields carry `#[non_exhaustive]`. Split files by capability (`model.rs`, `<feature>_view.rs`), not by global `views/` or `models/` roles.

Checklist: state and side-effect ownership explicit; `RenderOnce` vs `Entity<T>` deliberate; repeated elements keyed by domain ids; theme tokens and size tiers instead of literals; keyboard, focus, disabled and overlay behavior work together; loading, empty, error and cancellation paths exist; long collections virtualized; tests prove the behavior at the right layer; fmt, Clippy and targeted tests pass.

## 11. Gaps and corrections in this version
- `ThemeMode::System` does not exist. Use `Theme::sync_system_appearance(Some(window), cx)` with `cx.observe_window_appearance`.
- `Modal`, `Confirm`, `Toast`, `Divider`, `TextInput`, `TextArea`, a `VirtualList` component type, and `z_index` do not exist: use `Dialog`/`AlertDialog`, `Notification`, `Separator`, `Input`, `Textarea`, `v_virtual_list`/`h_virtual_list`, sibling paint order.
- `Sizable::medium()` does not exist; `Medium` is the default, only `xsmall`, `small`, `large`, `with_size` are defined (`crates/component/src/sizing.rs:190`).
- A free `label("text")` helper does not exist (`skills/gpui-kit/references/usage.md` is stale there): use `Label::new(..)` or `div().child(..)`.
- `Notification::new("text")` is invalid: `Notification::new()` takes no arguments. Use `Notification::info("text")`, `.success(..)`, `.warning(..)`, `.error(..)` (`crates/component/src/notification.rs:174,203`).
- `.tooltip(...)` on an arbitrary `div` is not public: the managed tooltip trait is `pub(crate)` (`crates/component/src/tooltip.rs:244`). Tooltips are component builders or `Tooltip::new(text).build(window, cx)` inside your own overlay.
- No Windows application icon: `WindowOptions::icon` is X11 only. Use an exe/installer resource.
- `Root::render_dialog_layer` / `render_notification_layer` were removed in 0.7.0; `Root` mounts the layers itself (`skills/gpui-kit/references/usage.md`), and that page still says `gpui-kit = "0.7"`, which predates 0.9.0.

## 12. Settings page with sidebar navigation (exact recipe)
Two verified routes. Use `setting::Settings` when the standard settings layout fits; assemble `Sidebar` yourself for custom content. The design guide calls this shell `sidebar workspace` and its content `Forms and settings` (`skills/gpui-kit-design-guides/references/design-guides.md:457,491`): use `Switch` for a setting that takes effect immediately, `Checkbox` for independent choices, `RadioGroup` for a small visible set, `Select` for a longer one.

```rust
use gpui_kit::component::{
    ActiveTheme as _, IconName, Theme, ThemeMode,
    setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings},
};
use gpui_kit::{AppContext as _, Context, Global, Render, SharedString, Window, div};

// Values live in a global so field readers and writers share one source of truth.
#[derive(Clone)]
pub struct Prefs { pub dark: bool, pub name: SharedString }
impl Global for Prefs {}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().bg(cx.theme().background).child(
            Settings::new("app-settings").sidebar_width(px(260.)).pages(vec![
                SettingPage::new("Appearance").icon(IconName::Settings2).default_open(true).group(
                    SettingGroup::new().title("Theme").item(
                        SettingItem::new("Dark mode", SettingField::switch(
                            |cx: &gpui_kit::App| cx.global::<Prefs>().dark,
                            |value: bool, cx: &mut gpui_kit::App| {
                                cx.update_global::<Prefs, _>(|prefs, _| prefs.dark = value);
                                Theme::change(if value { ThemeMode::Dark } else { ThemeMode::Light }, None, cx);
                            },
                        ))
                        .description("Saved with the profile, not the OS."),
                    ),
                ),
                SettingPage::new("Profile").group(
                    SettingGroup::new().title("Identity").item(
                        SettingItem::new("Display name", SettingField::input(
                            |cx: &gpui_kit::App| cx.global::<Prefs>().name.clone(),
                            |value: SharedString, cx: &mut gpui_kit::App| {
                                cx.update_global::<Prefs, _>(|prefs, _| prefs.name = value);
                            },
                        )),
                    ),
                ),
            ]),
        )
    }
}
```

`Settings` renders its own sidebar, page selection and search (`crates/component/src/setting/settings.rs:35`). Pages: `SettingPage::new(title)` with `.icon`, `.default_open(bool)`, `.group`/`.groups` (`setting/page.rs:34`). Groups: `SettingGroup::new().title(..).description(..).item(..)` (`setting/group.rs:36`). Items: `SettingItem::new(title, field).description(..).layout(Axis)` (`setting/item.rs:46`). Fields: `SettingField::switch/checkbox/input/dropdown/scrollable_dropdown/number_input/element/render`, each taking a `Fn(&App) -> T` reader and a `Fn(T, &mut App)` writer (`setting/fields/mod.rs:156-263`), which is why the values come from a `Global`.

Hand-built variant: copy `examples/sidebar/src/main.rs` — `h_flex().size_full()` with `Sidebar::new("sidebar").collapsible(SidebarCollapsible::Icon).collapsed(bool).header(SidebarHeader::new()..).child(SidebarGroup::new("Application").child(SidebarMenu::new().children([SidebarMenuItem::new("Appearance").icon(IconName::Settings2).active(selected == 0).on_click(..)])))`, then `v_flex().h_full().flex_1().min_w_0()` content switched on the selected index. Give the content column `h_full()` or the row `items_stretch()`.

## 13. Gotchas that make GPUI Kit code fail to compile
- Trait not in scope: `ActiveTheme as _` for `cx.theme()`; `button::ButtonVariants` for `.primary()`; `Sizable` for `.small()`; `Selectable` for `.selected(bool)`; `Disableable` for `.disabled(bool)`; `gpui_kit::prelude::FluentBuilder as _` for `.when(..)`; `ParentElement as _` for `.child(..)` in generic code; `component::scroll::ScrollableElement as _` for `.overflow_y_scrollbar()`.
- `Styled as _` is needed when a file does not glob-import `gpui_kit::*` but calls `.bg(..)`, `.p_4()`, `.w_full()`; `AppContext as _` is needed for `cx.new`, `cx.update_global`, `entity.update`.
- Element versus component: `Button` is an element; `Input` needs `Entity<InputState>`; `Dialog` is not a child element but opened through `window.open_dialog`, so passing a `Dialog` to `.child(..)` is a type error.
- `Entity<T>` is not an element: render the matching element (`Input::new(&state)`, `Select::new(&state)`, `DataTable::new(&state)`, `Slider::new(&state)`) or type-erase with `.into_any_element()`.
- `impl IntoElement` return position: a `match` or `if` returning two different element types does not compile; use `.into_any_element()` per arm (`examples/system_monitor/src/main.rs`) or `.map(|this| ..)`.
- `into_element()` and `into_any_element()` come from `IntoElement`, so import `gpui_kit::IntoElement` (the glob covers it). `.child("text")` converts internally, so strings need no conversion.
- Mutating without notifying renders nothing: call `cx.notify()` after the change, and never call it unconditionally inside `render`.
- `cx.subscribe` on a type without `impl EventEmitter<E>` does not compile; `cx.emit` likewise. Losing the `Subscription` (binding it to a local) silently disables the callback.
- Creating entities inside `open_dialog`/`open_sheet` builders loses their state, because the builder re-runs when the layer rebuilds (`crates/component/src/window_ext.rs:39`). Passing a `bool` to `Theme::change` does not compile; it takes `impl Into<ThemeMode>`.