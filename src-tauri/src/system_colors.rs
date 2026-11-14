use std::collections::HashMap;

// D-Bus requires a string array here; fixed-size Rust arrays encode as structures.
#[cfg(target_os = "linux")]
const PORTAL_GROUPS: &[&str] = &["org.freedesktop.appearance", "org.kde.kdeglobals.*"];

// Values stay typed until the frontend maps them to CSS variables.
#[derive(Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemColors {
    scheme: Option<u32>,
    accent: Option<[u8; 3]>,
    colors: HashMap<String, [u8; 3]>,
}

#[tauri::command]
pub async fn system_colors() -> Result<SystemColors, String> {
    #[cfg(target_os = "linux")]
    {
        read().await.map_err(|error| error.to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(SystemColors::default())
    }
}

#[cfg(target_os = "linux")]
async fn proxy(connection: &zbus::Connection) -> zbus::Result<zbus::Proxy<'_>> {
    zbus::Proxy::new(
        connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Settings",
    )
    .await
}

#[cfg(target_os = "linux")]
async fn read() -> zbus::Result<SystemColors> {
    use zbus::zvariant::OwnedValue;
    let connection = zbus::Connection::session().await?;
    let proxy = proxy(&connection).await?;
    let settings: HashMap<String, HashMap<String, OwnedValue>> =
        proxy.call("ReadAll", &(PORTAL_GROUPS,)).await?;
    let mut result = SystemColors::default();
    if let Some(appearance) = settings.get("org.freedesktop.appearance") {
        result.scheme = appearance
            .get("color-scheme")
            .and_then(|v| u32::try_from(v).ok());
        result.accent = appearance.get("accent-color").and_then(|v| {
            let (r, g, b) = <(f64, f64, f64)>::try_from(v.try_clone().ok()?).ok()?;
            [r, g, b]
                .iter()
                .all(|c| c.is_finite() && (0.0..=1.0).contains(c))
                .then(|| [r, g, b].map(|c| (c * 255.0).round() as u8))
        });
    }
    for (name, group, key) in [
        ("window", "Window", "BackgroundNormal"),
        ("text", "Window", "ForegroundNormal"),
        ("view", "View", "BackgroundNormal"),
        ("viewText", "View", "ForegroundNormal"),
        ("viewInactiveText", "View", "ForegroundInactive"),
        ("button", "Button", "BackgroundNormal"),
        ("selection", "Selection", "BackgroundNormal"),
        ("selectionText", "Selection", "ForegroundNormal"),
        ("disabled", "Window", "ForegroundInactive"),
        ("link", "View", "ForegroundLink"),
    ] {
        if let Some(color) = settings
            .get(&format!("org.kde.kdeglobals.Colors:{group}"))
            .and_then(|g| g.get(key))
            .and_then(|v| <&str>::try_from(v).ok())
            .and_then(parse_color)
        {
            result.colors.insert(name.to_owned(), color);
        }
    }
    Ok(result)
}

fn parse_color(value: &str) -> Option<[u8; 3]> {
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() != 6 || !hex.is_ascii() {
            return None;
        }
        return Some([
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
        ]);
    }
    let channels: Vec<u8> = value
        .split(',')
        .map(|c| c.trim().parse())
        .collect::<Result<_, _>>()
        .ok()?;
    channels.try_into().ok()
}

#[cfg(target_os = "linux")]
pub fn watch(app: tauri::AppHandle) {
    use futures_util::StreamExt;
    use tauri::Emitter;
    tauri::async_runtime::spawn(async move {
        let result: zbus::Result<()> = async {
            let connection = zbus::Connection::session().await?;
            let proxy = proxy(&connection).await?;
            let mut changes = proxy.receive_signal("SettingChanged").await?;
            while let Some(message) = changes.next().await {
                if let Ok((group, _, _)) =
                    message
                        .body()
                        .deserialize::<(String, String, zbus::zvariant::OwnedValue)>()
                {
                    if group == "org.freedesktop.appearance"
                        || group.starts_with("org.kde.kdeglobals.")
                    {
                        let _ = app.emit("system-colors-changed", ());
                    }
                }
            }
            Ok(())
        }
        .await;
        if let Err(error) = result {
            eprintln!("System appearance monitoring unavailable: {error}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::parse_color;
    #[cfg(target_os = "linux")]
    #[test]
    fn portal_groups_have_a_dbus_array_signature() {
        use zbus::zvariant::DynamicType;
        assert_eq!((super::PORTAL_GROUPS,).signature().to_string(), "(as)");
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires a running KDE desktop portal"]
    fn reads_live_kde_palette() {
        let colors = tauri::async_runtime::block_on(super::read()).expect("read desktop palette");
        assert!(
            colors.colors.contains_key("window"),
            "missing window palette"
        );
    }

    #[test]
    fn parses_only_valid_rgb_colors() {
        assert_eq!(parse_color("12, 34,56"), Some([12, 34, 56]));
        assert_eq!(parse_color("#21171a"), Some([33, 23, 26]));
        assert_eq!(parse_color("#F0B3C4"), Some([240, 179, 196]));
        for value in [
            "", "1,2", "1,2,3,4", "256,0,0", "-1,0,0", "red", "#123", "#gggggg",
        ] {
            assert_eq!(parse_color(value), None);
        }
    }
}
