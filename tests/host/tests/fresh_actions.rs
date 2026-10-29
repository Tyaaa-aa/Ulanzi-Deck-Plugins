//! A key that just got an action stores the parameters its rows show (the app's defaults); every
//! action must work with them once the user picked what the action is for.

use ulanzi_core::plugin::{FieldKind, Manifest};

#[test]
fn fresh_actions_work_out_of_the_box() {
    for (name, text) in [
        ("obs", include_str!("../../../plugins/obs/plugin.toml")),
        ("ha", include_str!("../../../plugins/ha/plugin.toml")),
    ] {
        let manifest = Manifest::parse(text).expect("manifest");
        for a in &manifest.actions {
            let mut t = a.defaults();
            // What the user picks or types: an entity, a scene, a name.
            for p in &a.params {
                if matches!(p.kind, FieldKind::Text | FieldKind::Item) && !t.contains_key(&p.key) {
                    let v = match (p.key.as_str(), a.id.as_str()) {
                        ("entity_id", "run") => "scene.test",
                        ("entity_id", "climate") => "climate.test",
                        ("entity_id", _) => "light.test",
                        _ => "test",
                    };
                    t.insert(p.key.clone(), toml::Value::String(v.into()));
                }
            }
            let params = serde_json::to_value(&t).expect("json");
            let r = match name {
                "obs" => ulanzi_plugin_obs::actions::parse_action(&a.id, &params).map(|_| ()),
                _ => ulanzi_plugin_ha::actions::plan(
                    &a.id,
                    &params,
                    &ulanzi_plugin_ha::protocol::Tracked::default(),
                )
                .map(|_| ()),
            };
            // Actions the plugin handles itself (reconnect, forget the password) are not the
            // parser's; any other failure is a gap in what a new action stores.
            if r.as_ref().is_err_and(|e| e.starts_with("unknown action")) && a.params.is_empty() {
                continue;
            }
            assert!(r.is_ok(), "{name}/{}: {:?} with {t:?}", a.id, r);
        }
    }
}
