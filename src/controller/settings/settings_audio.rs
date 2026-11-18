use crate::models::settings::*;
use crate::mapper::settings::*;
use crate::app::music_app::MusicApp;
use crate::models::app::Msg;
use crate::models::settings::SettingsMsg;
use crate::commands::audio::{audio_command, wasapi_command};

pub(crate) fn caps_of(c: wasapi_command::WasapiDeviceCapabilities) -> OutputCaps {
    OutputCaps { rates: c.exclusive_rates, bits: c.exclusive_bit_depths, mix_rate: c.mix_rate, mix_bits: c.mix_bit_depth, mix_channels: c.mix_channels }
}

impl MusicApp {
    /// Lit la qualité, le Replay Gain et sonde les sorties en exclusif (Windows).
    pub(crate) fn settings_audio_load(&mut self) {
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let q = audio_command::get_audio_quality_status(app.state()).await;
                let rg = audio_command::get_replay_gain_settings(app.state()).await;
                (q, rg)
            },
            |me, (q, rg)| {
                let (Ok(q), Ok(rg)) = (q, rg) else { return false };
                me.views.settings.audio.status = Some(AudioStatus {
                    setting: format!("{:?}", q.setting).to_lowercase(),
                    resolved: format!("{:?}", q.resolved).to_lowercase(),
                    virt: q.virt_kind,
                    cores: q.cpu_cores,
                    rg_mode: rg.mode,
                    preamp: rg.preamp_db,
                });
                me.views.settings.audio.preamp = None;
                true
            },
        );
        self.settings_probe_outputs();
    }

    pub(crate) fn settings_probe_outputs(&mut self) {
        if !WINDOWS {
            return;
        }
        let targets: Vec<(String, String)> = self
            .data
            .devices
            .iter()
            .filter(|d| !self.views.settings.audio.caps.contains_key(&d.display_name))
            .filter_map(|d| d.wasapi_id.clone().map(|id| (d.display_name.clone(), id)))
            .collect();
        if targets.is_empty() {
            return;
        }
        self.backend.call(
            async move {
                targets
                    .into_iter()
                    .filter_map(|(name, id)| wasapi_command::wasapi_probe_device_capabilities(id).ok().map(|c| (name, caps_of(c))))
                    .collect::<Vec<_>>()
            },
            |me, probes| {
                me.views.settings.audio.caps.extend(probes);
                true
            },
        );
    }

    pub(crate) fn settings_audio_update(&mut self, msg: SettingsMsg) -> bool {
        match msg {
            SettingsMsg::RefreshDevices => {
                self.views.settings.audio.caps.clear();
                self.views.settings.audio.refreshing = true;
                self.backend.call(
                    async move { crate::commands::audio::device_command::get_output_devices().await },
                    |me, devices| {
                        if let Ok(d) = devices {
                            me.data.devices = d;
                        }
                        me.views.settings.audio.refreshing = false;
                        me.settings_probe_outputs();
                        true
                    },
                );
            }
            SettingsMsg::Exclusive(on) => {
                if self.data.flag("wasapi_exclusive") != on {
                    return self.handle(Msg::ToggleExclusive);
                }
                return false;
            }
            SettingsMsg::Quality(v) => {
                let app = self.backend.app.clone();
                self.backend.call(
                    async move { audio_command::set_audio_quality_setting(app.state(), v.to_string()).await },
                    |me, r| {
                        let (Ok(q), Some(s)) = (r, me.views.settings.audio.status.as_mut()) else { return false };
                        s.setting = format!("{:?}", q.setting).to_lowercase();
                        s.resolved = format!("{:?}", q.resolved).to_lowercase();
                        true
                    },
                );
            }
            SettingsMsg::ReplayGain(mode) => {
                let preamp = self.views.settings.audio.status.as_ref().map(|s| s.preamp).unwrap_or(0.0);
                self.save_replay_gain(mode.to_string(), preamp);
            }
            SettingsMsg::DacTest => {
                let Some(d) = self.data.devices.iter().find(|d| d.is_selected).or_else(|| self.data.devices.iter().find(|d| d.is_default)) else {
                    return false;
                };
                let Some(id) = d.wasapi_id.clone() else { return false };
                let a = &mut self.views.settings.audio;
                a.dac_device = Some(d.display_name.clone());
                a.dac_running = true;
                a.dac_caps = None;
                a.dac_error = None;
                self.backend.call(async move { wasapi_command::wasapi_probe_device_capabilities(id) }, |me, r| {
                    let a = &mut me.views.settings.audio;
                    a.dac_running = false;
                    match r {
                        Ok(c) => a.dac_caps = Some(caps_of(c)),
                        Err(e) => a.dac_error = Some(e),
                    }
                    true
                });
            }
            _ => return false,
        }
        true
    }

    /// Préampli : affiché pendant le glissement, enregistré au relâchement.
    pub(crate) fn settings_preamp(&mut self, db: f64, commit: bool) -> bool {
        let db = ((db * 2.0).round() / 2.0) as f32;
        self.views.settings.audio.preamp = Some(db);
        if commit {
            let mode = self.views.settings.audio.status.as_ref().map(|s| s.rg_mode.clone()).unwrap_or_else(|| "off".into());
            self.save_replay_gain(mode, db);
        }
        true
    }

    pub(crate) fn save_replay_gain(&mut self, mode: String, preamp: f32) {
        let app = self.backend.app.clone();
        self.backend.call(
            async move { audio_command::set_replay_gain_settings(app.state(), mode, preamp).await },
            |me, r| {
                let (Ok(rg), Some(s)) = (r, me.views.settings.audio.status.as_mut()) else { return false };
                s.rg_mode = rg.mode;
                s.preamp = rg.preamp_db;
                me.views.settings.audio.preamp = None;
                true
            },
        );
    }
}
