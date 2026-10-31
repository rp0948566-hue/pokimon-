//! Measured GPU use during one analysis run, sampled from `nvidia-smi` (absent tool or no NVIDIA
//! GPU means no sample, never a made-up number).
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GpuSample {
    pub peak_vram_mib: u64,
    pub peak_util_pct: u32,
    pub mean_util_pct: f32,
    pub samples: u32,
}

pub struct GpuSampler {
    stop: Arc<AtomicBool>,
    data: Arc<Mutex<Vec<(u64, u32)>>>,
}

fn read_once() -> Option<(u64, u32)> {
    let out = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=memory.used,utilization.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    parse(&String::from_utf8_lossy(&out.stdout))
}

fn parse(text: &str) -> Option<(u64, u32)> {
    let mut f = text.lines().next()?.split(',');
    Some((
        f.next()?.trim().parse().ok()?,
        f.next()?.trim().parse().ok()?,
    ))
}

impl GpuSampler {
    pub fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let data = Arc::new(Mutex::new(Vec::new()));
        let (s, d) = (stop.clone(), data.clone());
        std::thread::spawn(move || {
            while !s.load(Ordering::Relaxed) {
                match read_once() {
                    Some(v) => d.lock().unwrap().push(v),
                    None => break,
                }
                std::thread::sleep(Duration::from_millis(250));
            }
        });
        Self { stop, data }
    }

    pub fn finish(self) -> Option<GpuSample> {
        self.stop.store(true, Ordering::Relaxed);
        let v = self.data.lock().ok()?;
        if v.is_empty() {
            return None;
        }
        Some(GpuSample {
            peak_vram_mib: v.iter().map(|x| x.0).max()?,
            peak_util_pct: v.iter().map(|x| x.1).max()?,
            mean_util_pct: v.iter().map(|x| x.1 as f32).sum::<f32>() / v.len() as f32,
            samples: v.len() as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_nvidia_smi_csv_and_rejects_junk() {
        assert_eq!(super::parse("2561, 37\n"), Some((2561, 37)));
        assert_eq!(super::parse("N/A, N/A"), None);
        assert_eq!(super::parse(""), None);
    }
}
