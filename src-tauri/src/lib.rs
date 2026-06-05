//! Tauri 데스크톱 백엔드 — invoke 핸들러가 프론트 호출을 axon-core 로 넘긴다.

mod commands;
mod daemon;
mod dart;
mod export;
mod ledger;
mod menu;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(daemon::Approvals::default())
        .manage(daemon::SessionCursor::default())
        .manage(ledger::LedgerState::default())
        .menu(menu::build)
        .on_menu_event(menu::on_event)
        .invoke_handler(tauri::generate_handler![
            dart::keychain::save_opendart_key,
            dart::keychain::has_opendart_key,
            dart::keychain::delete_opendart_key,
            dart::opendart::fetch_financials,
            dart::opendart::fetch_financials_range,
            dart::opendart::fetch_ratios,
            dart::opendart::fetch_footing,
            dart::taxonomy::load_taxonomy,
            dart::taxonomy::fetch_section_map,
            dart::xbrl::load_calc_tree,
            dart::xbrl::consensus_tree,
            dart::xbrl::fetch_calc_footing,
            ledger::ledger_open,
            ledger::ledger_import,
            ledger::ledger_preview,
            ledger::ledger_op,
            ledger::ledger_counterpart,
            ledger::ledger_sample,
            ledger::ledger_list_datasets,
            ledger::ledger_get_dataset,
            ledger::ledger_set_active,
            ledger::ledger_delete_dataset,
            ledger::ledger_rename_dataset,
            dart::corp::search_corp,
            export::command::export_table,
            daemon::config::save_daemon_url,
            daemon::config::get_daemon_url,
            daemon::config::save_daemon_token,
            daemon::config::has_daemon_token,
            daemon::config::clear_daemon,
            daemon::config::ping_daemon,
            daemon::auth::daemon_login,
            daemon::auth::daemon_logout,
            daemon::entitlement::fetch_me,
            daemon::entitlement::daemon_token_claims,
            daemon::turn::daemon_send,
            daemon::turn::daemon_cancel,
            daemon::turn::daemon_approve,
            daemon::policy::set_workspace,
            daemon::policy::get_workspace,
            daemon::policy::clear_workspace,
            daemon::policy::set_shell_enabled,
            daemon::policy::shell_is_enabled,
            daemon::fin_context::set_fin_context,
            daemon::fin_context::clear_fin_context,
            daemon::fin_context::get_fin_context,
        ])
        .run(tauri::generate_context!())
        .expect("tauri 앱 실행 실패");
}
