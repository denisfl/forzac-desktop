//! Whether the desktop prefers a dark style: `org.freedesktop.appearance color-scheme` from the settings portal (and
//! its changes), else GTK's `gtk-application-prefer-dark-theme`.

use gtk::{gio, glib, prelude::*};

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PATH: &str = "/org/freedesktop/portal/desktop";
const SETTINGS: &str = "org.freedesktop.portal.Settings";
const NAMESPACE: &str = "org.freedesktop.appearance";
const KEY: &str = "color-scheme";

/// How long the start waits for the portal; without an answer GTK's setting decides.
const TIMEOUT_MS: i32 = 200;

fn bus() -> Option<gio::DBusConnection> {
    gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).ok()
}

/// A `color-scheme` value: 1 prefers dark, 2 light, 0 has no preference.
fn is_dark(value: &glib::Variant) -> Option<bool> {
    // `ReadOne` answers with the value in a `v`; the older `Read` with a `v` in a `v`.
    let mut value = value.clone();
    while value.is_type(glib::VariantTy::VARIANT) {
        let Some(inner) = value.as_variant() else { break };
        value = inner;
    }
    value.get::<u32>().map(|scheme| scheme == 1)
}

pub fn dark() -> bool {
    let portal = bus().and_then(|bus| {
        let read = |method: &str| {
            let reply_type = glib::VariantTy::new("(v)").ok()?;
            let args = (NAMESPACE, KEY).to_variant();
            bus.call_sync(
                Some(PORTAL),
                PATH,
                SETTINGS,
                method,
                Some(&args),
                Some(reply_type),
                gio::DBusCallFlags::NONE,
                TIMEOUT_MS,
                gio::Cancellable::NONE,
            )
            .ok()
        };
        let reply = read("ReadOne").or_else(|| read("Read"))?;
        is_dark(&reply.child_value(0))
    });
    portal.unwrap_or_else(|| gtk::Settings::default().is_some_and(|s| s.is_gtk_application_prefer_dark_theme()))
}

/// Calls `changed` when the desktop's preference changes, until the returned subscription is dropped.
pub fn watch(changed: impl Fn(bool) + 'static) -> Option<gio::SignalSubscription> {
    let bus = bus()?;
    Some(bus.subscribe_to_signal(
        Some(PORTAL),
        Some(SETTINGS),
        Some("SettingChanged"),
        Some(PATH),
        Some(NAMESPACE),
        gio::DBusSignalFlags::NONE,
        move |signal| {
            let p = signal.parameters;
            if p.n_children() == 3
                && p.child_value(1).str() == Some(KEY)
                && let Some(dark) = is_dark(&p.child_value(2))
            {
                changed(dark);
            }
        },
    ))
}
