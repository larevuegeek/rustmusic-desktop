//! Progression d'un import ou d'un scan : chargeur de la bibliothèque et barre d'état.
use crate::models::app::{Data, ScanProgress};
use crate::models::library::ImportProgressView;
use karbon_ui::intl::{self, t, t_with};
use std::time::Duration;

/// « 1:05 », « 12:40 ».
fn minutes_seconds(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

pub(crate) fn import_progress(data: &Data, elapsed: Duration) -> Option<ImportProgressView> {
    let s: &ScanProgress = data.scan.as_ref()?;
    let percent = if s.total > 0 { (s.current as f32 * 100. / s.total as f32).clamp(0., 100.) } else { 0. };
    let title = if s.stopping {
        t("folders_popin.stopping")
    } else if s.listing {
        t("system.task_listing")
    } else if s.total > 0 {
        t("system.importing")
    } else {
        t("system.preparing")
    };
    let library_name = data
        .libraries
        .iter()
        .find(|l| Some(l.id) == s.library_id.or(data.library_id))
        .map(|l| l.name.clone())
        .unwrap_or_default();
    let task = if s.listing {
        t("system.task_listing")
    } else if s.import {
        t("system.task_importing")
    } else {
        t_with("system.task_rescan", &[("name", &library_name)])
    };
    let counter = if s.listing {
        let key = if s.found == 1 { "system.files_found_one" } else { "system.files_found_n" };
        t_with(key, &[("n", &intl::number(s.found))])
    } else if s.total > 0 {
        format!("{} / {}", intl::number(s.current), intl::number(s.total))
    } else {
        String::new()
    };
    let seconds = elapsed.as_secs();
    let eta = if s.current > 0 && s.total > s.current && seconds > 0 && !s.listing {
        let remaining = (s.total - s.current) as f64 * elapsed.as_secs_f64() / s.current as f64;
        if remaining < 5. {
            t("system.almost_done")
        } else {
            t_with("system.time_left", &[("time", &minutes_seconds(remaining as u64))])
        }
    } else {
        String::new()
    };
    // Comme la 0.4 : le chargeur prend la place des pages tant que la bibliothèque
    // importée n'a encore aucun morceau.
    let empty = data
        .libraries
        .iter()
        .find(|l| Some(l.id) == s.library_id.or(data.library_id))
        .is_none_or(|l| l.total_tracks == 0);
    Some(ImportProgressView {
        import: s.import,
        listing: s.listing,
        stopping: s.stopping,
        percent,
        percent_label: format!("{}", percent.round() as i64),
        title,
        task,
        icon: if s.listing {
            "lucide:folder-search"
        } else if s.import {
            "lucide:hard-drive-download"
        } else {
            "lucide:refresh-cw"
        },
        detail: if s.listing { counter.clone() } else { s.file_name.clone() },
        counter,
        elapsed: if seconds > 0 { minutes_seconds(seconds) } else { String::new() },
        eta,
        directory: s.directory.clone(),
        file_name: s.file_name.clone(),
        loader: s.import && empty,
        library_id: s.library_id.or(data.library_id),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_then_import_with_remaining_time() {
        let mut data = Data::default();
        assert!(import_progress(&data, Duration::ZERO).is_none());
        data.scan = Some(ScanProgress { import: true, listing: true, found: 312, ..Default::default() });
        let v = import_progress(&data, Duration::from_secs(3)).unwrap();
        assert!(v.listing && v.loader);
        assert!(v.counter.contains("312"));
        assert!(v.eta.is_empty());
        data.scan = Some(ScanProgress { import: true, current: 100, total: 400, ..Default::default() });
        let v = import_progress(&data, Duration::from_secs(60)).unwrap();
        assert_eq!(v.percent_label, "25");
        assert_eq!(v.elapsed, "1:00");
        assert!(v.eta.contains("3:00"), "{}", v.eta);
    }
}
