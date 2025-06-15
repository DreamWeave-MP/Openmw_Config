#[macro_export]
macro_rules! impl_singleton_setting {
    ($($variant:ident => {
        get: $get_fn:ident,
        set: $set_fn:ident,
        in_type: $in_type:ident
    }),* $(,)?) => {
            $(
                pub fn $get_fn(&self) -> Option<&$in_type> {
                    self.settings.iter().find_map(|setting| {
                        match setting {
                            SettingValue::$variant(value) => Some(value),
                            _ => None,
                        }
                    })
                }

                pub fn $set_fn(&mut self, new: Option<$in_type>) {
                    let index = self
                        .settings
                        .iter()
                        .position(|setting| matches!(setting, SettingValue::$variant(_)));

                    match (index, new) {
                        (Some(i), Some(value)) => self.settings[i] = SettingValue::$variant(value),
                        (None, Some(value)) => self.settings.push(SettingValue::$variant(value)),
                        (Some(i), None) => { self.settings.remove(i); }
                        (None, None) => {}
                    }
                }
            )*
    };
}
