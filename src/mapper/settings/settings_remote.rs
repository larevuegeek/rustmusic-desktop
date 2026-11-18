use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::mapper::settings::toggle_msg;
use crate::models::settings::Control;
use crate::models::settings::Ghost;
use crate::models::settings::GroupAction;
use crate::models::settings::OptionGroup;
use crate::models::settings::OptionRow;
use crate::models::settings::SettingsMsg;
use crate::core::amp_remote::model::{Amp, Brand};

pub fn brand_label(b: &Brand) -> &'static str {
    match b {
        Brand::Onkyo => "Onkyo · Pioneer · Integra",
        Brand::Denon => "Denon · Marantz",
        Brand::Yamaha => "Yamaha",
    }
}

impl MusicApp {
    pub(crate) fn settings_remote(&self) -> Vec<OptionGroup> {
        let active = self.data.flag("amp_remote");
        let mut groups = vec![self
            .option_group(
                "amp_remote",
                Some(t("amp_remote.group")),
                Some(t("amp_remote.group_hint")),
                Some("amp_remote.group"),
                vec![self.keep(
                    OptionRow::new("amp_remote", t("amp_remote.toggle"), t("amp_remote.toggle_desc"), Control::Toggle(toggle_msg("amp_remote"), active)),
                    "onkyo pioneer integra denon marantz yamaha ampli télécommande remote",
                )],
            )
            .with_action(GroupAction::Badge(t("settings.beta_badge"), "amber"))];
        if active {
            let r = &self.views.settings.remote;
            let mut lines = Vec::new();
            if let Some((amp, _)) = &r.pilot {
                lines.push(self.keep(OptionRow::block("amp_pilot", Control::Block("amp-pilot")), &amp.name));
            }
            // L'ampli piloté reste listé même s'il n'a pas répondu à cette recherche.
            let others: Vec<&Amp> = r.found.iter().filter(|a| r.pilot.as_ref().is_none_or(|(p, _)| p.id != a.id)).collect();
            for amp in &others {
                lines.push(self.keep(
                    OptionRow::new(
                        &amp.id,
                        amp.name.clone(),
                        format!("{} · {} · {}", brand_label(&amp.brand), amp.model, amp.ip),
                        Control::Buttons(vec![Ghost::new(
                            "material-symbols:settings-remote-outline-rounded",
                            t("amp_remote.control"),
                            Msg::Settings(SettingsMsg::AmpSelect(Some((*amp).clone()))),
                        )]),
                    ),
                    "",
                ));
            }
            if others.is_empty() && r.pilot.is_none() && r.searched && !r.searching {
                lines.push(self.keep(OptionRow::new("amp_none", String::new(), t("amp_remote.none"), Control::Block("amp-none")), &t("amp_remote.none")));
            }
            let button = Ghost::new(
                "material-symbols:refresh-rounded",
                if r.searching { t("network_player.searching") } else { t("network_player.search") },
                Msg::Settings(SettingsMsg::AmpSearch),
            )
            .busy(r.searching);
            groups.push(
                self.option_group("amps", Some(t("amp_remote.amps")), Some(t("amp_remote.amps_hint")), Some("amp_remote.amps"), lines)
                    .with_action(GroupAction::Button(button)),
            );
        }
        groups
    }
}
