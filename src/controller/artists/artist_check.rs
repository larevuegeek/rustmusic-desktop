use crate::models::artists::*;
use serde_json::Value;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::models::layout::PopinSize;
use crate::commands::library::artist_split_command;

pub(crate) fn rule(v: &Value) -> RuleView {
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
    RuleView { name: s("name"), source: s("source"), parts: v.get("parts").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default() }
}

impl MusicApp {
    pub(crate) fn artist_check_open(&mut self) {
        self.menu = None;
        self.open_popin(Popin {
            title: t("artist_split.report_title"),
            icon: Some("material-symbols:group-outline-rounded"),
            body: PopinBody::ArtistCheck,
            size: PopinSize::Lg,
            bare: false,
            flush: true,
        });
    }

    pub(crate) fn artist_check_update(&mut self, msg: ArtistCheckMsg) -> bool {
        match msg {
            ArtistCheckMsg::Start => {
                self.artist_check_open();
                if self.views.artist_check.running {
                    return true;
                }
                self.views.artist_check = ArtistCheck { running: true, started: Some(std::time::Instant::now()), ..Default::default() };
                let app = self.backend.app.clone();
                self.backend.call(async move { artist_split_command::artist_check_start(app.clone(), app.state()).await }, |me, r| {
                    if let Err(e) = r {
                        if e != "busy" {
                            me.views.artist_check.running = false;
                            me.views.artist_check.error = Some(e);
                        }
                    }
                    true
                });
            }
            ArtistCheckMsg::One(name) => {
                self.artist_check_open();
                if self.views.artist_check.running {
                    return true;
                }
                self.views.artist_check = ArtistCheck { running: true, total: 1, current: name.clone(), one: Some(name.clone()), started: Some(std::time::Instant::now()), ..Default::default() };
                let app = self.backend.app.clone();
                self.backend.call(async move { artist_split_command::artist_check_name(app.state(), name).await }, |me, r| {
                    let a = &mut me.views.artist_check;
                    a.running = false;
                    match r {
                        Ok(run) => {
                            a.run_id = Some(run);
                            me.artist_load_report(Some(run));
                        }
                        Err(e) => a.error = Some(e),
                    }
                    true
                });
            }
            ArtistCheckMsg::Report => {
                self.artist_check_open();
                if !self.views.artist_check.running {
                    let run = self.views.artist_check.run_id;
                    self.artist_load_report(run);
                }
            }
            ArtistCheckMsg::Close => {
                self.views.popin = None;
                if self.views.artist_check.touched {
                    self.views.artist_check.touched = false;
                    self.load_library_data();
                }
            }
            ArtistCheckMsg::Undo(name) => return self.artist_rule(name, false, true),
            ArtistCheckMsg::Redo(name) => return self.artist_rule(name, true, true),
            ArtistCheckMsg::UndoAll => {
                let names: Vec<String> = self.views.artist_check.report.iter().flat_map(|r| r.split.iter()).filter(|v| !self.views.artist_check.undone.contains(&v.name)).map(|v| v.name.clone()).collect();
                for n in names {
                    self.artist_rule(n, false, true);
                }
            }
            ArtistCheckMsg::Decide(name, split) => return self.artist_rule(name, split, false),
        }
        true
    }

    /// `rule` : enregistre la décision, puis note l'annulation ou la décision prise.
    pub(crate) fn artist_rule(&mut self, name: String, split: bool, undo: bool) -> bool {
        let parts: Vec<String> = if split {
            let r = self.views.artist_check.report.as_ref();
            r.and_then(|r| r.split.iter().chain(r.unknown.iter()).find(|v| v.name == name)).map(|v| v.parts.clone()).unwrap_or_default()
        } else {
            Vec::new()
        };
        self.views.artist_check.working = true;
        self.views.artist_check.fault = None;
        let app = self.backend.app.clone();
        let (n, decision) = (name.clone(), if split { "split" } else { "whole" }.to_string());
        self.backend.call(async move { artist_split_command::artist_rule_set(app.state(), n, decision, parts).await }, move |me, r| {
            let a = &mut me.views.artist_check;
            a.working = false;
            match r {
                Ok(()) => {
                    a.touched = true;
                    if undo {
                        if split {
                            a.undone.remove(&name);
                        } else {
                            a.undone.insert(name);
                        }
                    } else {
                        a.decided.insert(name, if split { "split" } else { "whole" }.into());
                    }
                }
                Err(e) => a.fault = Some(e),
            }
            true
        });
        true
    }

    pub(crate) fn artist_load_report(&mut self, run: Option<i64>) {
        let app = self.backend.app.clone();
        self.backend.call(
            async move { artist_split_command::artist_check_report(app.state(), run).await.map(|r| serde_json::to_value(r).unwrap_or_default()) },
            |me, r| {
                match r {
                    Ok(v) => {
                        let list = |k: &str| v.get(k).and_then(Value::as_array).map(|a| a.iter().map(rule).collect::<Vec<_>>()).unwrap_or_default();
                        let run = v.get("run").filter(|r| !r.is_null());
                        if let Some(imgs) = v.get("images").and_then(Value::as_object) {
                            for (k, p) in imgs {
                                if let Some(p) = p.as_str() {
                                    me.views.artist_check.images.insert(k.clone(), p.to_string());
                                }
                            }
                        }
                        me.views.artist_check.report = Some(Report {
                            checked: run.and_then(|r| r.get("checked")).and_then(Value::as_i64),
                            error: run.and_then(|r| r.get("error")).is_some_and(|e| !e.is_null()),
                            split: list("split"),
                            whole: v.get("whole").and_then(Value::as_array).map_or(0, |a| a.len()),
                            unknown: list("unknown"),
                            run_unknown: v.get("run_unknown").and_then(Value::as_i64).unwrap_or(0),
                        });
                    }
                    Err(e) => me.views.artist_check.fault = Some(e),
                }
                true
            },
        );
    }

    /// Événements du moteur.
    pub(crate) fn artist_event(&mut self, name: &str, payload: &Value) -> bool {
        match name {
            "artist-check-progress" => {
                let a = &mut self.views.artist_check;
                let s = |k: &str| payload.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
                a.running = true;
                a.started.get_or_insert_with(std::time::Instant::now);
                a.done = payload.get("done").and_then(Value::as_i64).unwrap_or(a.done);
                a.total = payload.get("total").and_then(Value::as_i64).unwrap_or(a.total);
                let decision = s("decision");
                if decision.is_empty() {
                    a.current = s("name");
                } else {
                    let parts = payload.get("parts").and_then(Value::as_array).map(|x| x.iter().filter_map(|p| p.as_str().map(str::to_string)).collect()).unwrap_or_default();
                    a.live.push(LiveItem { name: s("name"), decision, parts });
                }
                if let Some(imgs) = payload.get("images").and_then(Value::as_object) {
                    for (k, p) in imgs {
                        if let Some(p) = p.as_str() {
                            a.images.insert(k.clone(), p.to_string());
                        }
                    }
                }
                true
            }
            "artist-check-done" => {
                let run = payload.get("run_id").and_then(Value::as_i64);
                let a = &mut self.views.artist_check;
                a.running = false;
                a.run_id = run;
                a.error = payload.get("error").and_then(Value::as_str).map(str::to_string);
                let open = self.views.popin.as_ref().is_some_and(|p| matches!(p.body, PopinBody::ArtistCheck));
                // Des noms à revoir : la fenêtre s'ouvre (`finished`).
                if !open && self.views.artist_check.live.iter().any(|x| x.decision != "whole") {
                    self.artist_check_open();
                }
                self.artist_load_report(run);
                true
            }
            _ => false,
        }
    }
}
