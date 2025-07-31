/*

Module that sends out current cpu and memory usage.

*/

use axum::{response::{Json, IntoResponse}, routing::get, Router};
use serde::Serialize;
use std::fs;
use std::process::Command;
use tokio;
use tokio::sync::Mutex;
use sysinfo::{CpuRefreshKind, RefreshKind, System};

use once_cell::sync::Lazy;

#[derive(Serialize)]
struct SystemUsage {
    cpu_usage: Vec<f32>, // One entry per core
    ram_usage: f32,
    swap_usage: f32,
}
static SYS: Lazy<Mutex<System>> = Lazy::new(|| {
    Mutex::new(System::new_with_specifics(
        RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
    ))
});
async fn get_cpu_usage() -> Vec<f32> {
    let mut sys = SYS.lock().await;

    sys.refresh_cpu_usage();

    sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect()
}

async fn get_memory_usage() -> f32 {
    let mut sys = System::new_all();
    sys.refresh_memory();

    let total = sys.total_memory() as f32;
    let free = sys.available_memory() as f32;
    let used = total - free;

    if total > 0.0 {
        (used / total) * 100.0
    } else {
        0.0
    }
}

async fn get_swap_usage() -> f32 {
    let mut sys = System::new_all();
    sys.refresh_memory();

    let swap_total = sys.total_swap() as f32;
    let swap_free = sys.free_swap() as f32;
    let swap_used = swap_total - swap_free;

    if swap_total > 0.0 {
        (swap_used / swap_total) * 100.0
    } else {
        0.0
    }
}



async fn get_system_usage() -> SystemUsage {
    let cpu_usage = get_cpu_usage().await;
    let ram_usage = get_memory_usage().await;
    let swap_usage = get_swap_usage().await;

    SystemUsage {
        cpu_usage,
        ram_usage,
        swap_usage,
    }
}

pub async fn system_usage_handler() -> impl IntoResponse {
    let usage = get_system_usage().await;
    Json(usage)
}
