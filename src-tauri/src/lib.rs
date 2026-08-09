// 分层入口：命令层（薄）→ 引擎（纯逻辑）→ db（数据），见 docs/technical-plan.md §2、§4.1
pub mod commands;
pub mod db;
pub mod engine;

use commands::AppPaths;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // S1：路径解析（环境变量覆盖）→ 初始化 db（三表迁移）→ 注入 state
            let paths = AppPaths::resolve()?;
            let db = engine::init_db(&paths.data_dir.join("skills-keeper.db"))?;
            app.manage(paths);
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_cmds::list_skills,
            commands::scan_cmds::scan,
            commands::scan_cmds::get_status_matrix,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
