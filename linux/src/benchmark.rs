//! Benchmarks locais, curtos e reproduzíveis. Os resultados não são equivalentes ao RESET BENCH original.
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct Results {
    pub single_mops: f64,
    pub multi_mops: f64,
    pub threads: usize,
    pub memory_gib_s: f64,
    pub memory_mib: usize,
}

fn cpu_rate(threads: usize, duration: Duration) -> Result<f64, String> {
    let start = Instant::now();
    let mut workers = Vec::new();
    for id in 0..threads {
        match std::thread::Builder::new()
            .name(format!("reset-bench-{id}"))
            .spawn(move || {
                let end = Instant::now() + duration;
                let mut state = black_box(0x9e3779b97f4a7c15u64 ^ id as u64);
                let mut operations = 0u64;
                while Instant::now() < end {
                    for _ in 0..4096 {
                        state ^= state >> 12;
                        state ^= state << 25;
                        state ^= state >> 27;
                        state = state.wrapping_mul(0x2545f4914f6cdd1d);
                    }
                    operations += 4096;
                    black_box(state);
                }
                operations
            }) {
            Ok(handle) => workers.push(handle),
            Err(e) => {
                for worker in workers {
                    let _ = worker.join();
                }
                return Err(format!("Não foi possível iniciar threads: {e}"));
            }
        }
    }
    let mut operations = 0u64;
    for worker in workers {
        operations += worker
            .join()
            .map_err(|_| "Falha em uma thread do benchmark".to_owned())?;
    }
    Ok(operations as f64 / start.elapsed().as_secs_f64() / 1_000_000.0)
}

fn memory_rate(duration: Duration, memory_mib: usize) -> Result<f64, String> {
    let length = memory_mib
        .checked_mul(1024 * 1024 / 8)
        .ok_or("Tamanho de memória inválido")?;
    let mut memory = Vec::<u64>::new();
    memory
        .try_reserve_exact(length)
        .map_err(|_| "Não foi possível reservar memória para o teste")?;
    memory.resize(length, 0);
    let start = Instant::now();
    let mut passes = 0u64;
    while start.elapsed() < duration {
        memory.fill(black_box(passes));
        black_box(&memory);
        passes += 1;
    }
    let seconds = start.elapsed().as_secs_f64();
    Ok((passes as f64 * memory_mib as f64 / 1024.0) / seconds)
}

pub fn run(duration: Duration, memory_mib: usize) -> Result<Results, String> {
    if duration.is_zero() || memory_mib == 0 {
        return Err("Duração e tamanho devem ser positivos".into());
    }
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    Ok(Results {
        single_mops: cpu_rate(1, duration)?,
        multi_mops: cpu_rate(threads, duration)?,
        threads,
        memory_gib_s: memory_rate(duration, memory_mib)?,
        memory_mib,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_parameters_rejected() {
        assert!(run(Duration::ZERO, 1).is_err());
        assert!(run(Duration::from_millis(1), 0).is_err());
    }
    #[test]
    fn produces_positive_results() {
        let r = run(Duration::from_millis(20), 1).unwrap();
        assert!(r.single_mops > 0.0 && r.multi_mops > 0.0 && r.memory_gib_s > 0.0);
    }
}
