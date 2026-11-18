use crate::models::settings::*;
use crate::app::music_app::MusicApp;
use crate::commands::audio::wasapi_command;

impl MusicApp {
    pub(crate) fn device_update(&mut self, msg: DeviceMsg) -> bool {
        match msg {
            DeviceMsg::Open(i) => {
                let Some(d) = self.data.devices.get(i) else { return false };
                let cached = self.views.settings.audio.caps.contains_key(&d.display_name);
                let probe = (!cached).then(|| d.wasapi_id.clone().map(|id| (id, d.display_name.clone()))).flatten();
                self.views.device_details = Some(DeviceDetails { index: i, probing: probe.is_some(), error: None });
                if let Some((id, name)) = probe {
                    self.backend.call(async move { wasapi_command::wasapi_probe_device_capabilities(id) }, move |me, r| {
                        let Some(dd) = me.views.device_details.as_mut() else { return false };
                        dd.probing = false;
                        match r {
                            Ok(c) => {
                                me.views.settings.audio.caps.insert(name, crate::controller::settings::caps_of(c));
                            }
                            Err(e) => dd.error = Some(e),
                        }
                        true
                    });
                }
            }
            DeviceMsg::Close => self.views.device_details = None,
            DeviceMsg::Select => {
                let Some(dd) = self.views.device_details.take() else { return false };
                return self.handle(crate::models::app::Msg::ChooseOutput(dd.index));
            }
        }
        true
    }
}
