// probetrain: train the prompt-injection probe from the model's hidden states.
// Milestone 8 (Probe Depths). Session plan: SESSION-milestone8.md
//
// usage: probetrain <features.bin> [probe.bin]      (use --release: it's ~1M multiply-adds per row)
//
// A probe is a sensor inside the model: a weighted sum of one layer's hidden state,
// squashed to 0..1. Training it is logistic regression, the simplest classifier there is.
// probe_features.py wrote the data; this writes probe.bin for stream_probe.py to use live.

#![allow(unused_variables, dead_code)] // delete this line once all YOUR PARTs are done

use std::env;
use std::fs;

struct Row {
    example: u32,
    label: f32, // 1.0 = token is part of an injection, 0.0 = it isn't
    x: Vec<f32>,
}

fn sigmoid(z: f32) -> f32 {
    // YOUR PART 19: squash any number into 0..1.
    // sigmoid(z) = 1 / (1 + e^(-z))      (f32 has .exp())
    //
    // Done when: cargo test --bin probetrain probe_sigmoid
    todo!("sigmoid")
}

/// The probe's reading for one hidden state: sigmoid(w . x + b).
fn score(w: &[f32], b: f32, x: &[f32]) -> f32 {
    // YOUR PART 20:
    // 1. z = b + the sum of w[i] * x[i] over every i
    // 2. return sigmoid(z)
    //
    // Done when: cargo test --bin probetrain probe_score
    todo!("score")
}

/// One pass over the training rows. Returns the mean log loss (lower is better).
fn train_epoch(w: &mut [f32], b: &mut f32, rows: &[&Row], lr: f32) -> f32 {
    // YOUR PART 21: stochastic gradient descent for logistic regression.
    // For each row:
    //   1. p = score(w, *b, &row.x)
    //   2. g = p - row.label                (how wrong it was, with a sign)
    //   3. nudge every weight against the error: w[i] -= lr * g * row.x[i]
    //      and the bias:                       *b   -= lr * g
    //   4. add this row's loss to a total:
    //      loss = -(y * ln(p) + (1 - y) * ln(1 - p)),
    //      with p clamped to 1e-6..=1.0 - 1e-6 first so ln never sees 0
    // Return total loss / number of rows.
    //
    // Done when: cargo test --bin probetrain probe_learns
    todo!("train_epoch")
}

/// Confusion counts at a 0.5 threshold: (true pos, false pos, true neg, false neg).
fn evaluate(w: &[f32], b: f32, rows: &[&Row]) -> (usize, usize, usize, usize) {
    // YOUR PART 22: grade the probe on rows it never trained on.
    // For each row, predicted = score(...) >= 0.5, actual = row.label == 1.0, then count:
    //   predicted and actual       -> true positive
    //   predicted, not actual      -> false positive (a false alarm)
    //   not predicted, not actual  -> true negative
    //   not predicted, but actual  -> false negative (a missed injection)
    //
    // Done when: cargo test --bin probetrain probe_confusion
    todo!("evaluate")
}

// ---------- Pre-filled: file I/O, normalization, shuffling ----------

fn read_features(path: &str) -> (Vec<Row>, usize, u32) {
    let data = fs::read(path).unwrap_or_else(|e| panic!("could not read {path}: {e}"));
    assert_eq!(&data[0..4], b"PRB1", "{path} isn't a features file from probe_features.py");
    let u32_at = |i: usize| u32::from_le_bytes(data[i..i + 4].try_into().unwrap());
    let (n, dim, layer) = (u32_at(4) as usize, u32_at(8) as usize, u32_at(12));
    let mut rows = Vec::with_capacity(n);
    let mut off = 16;
    for _ in 0..n {
        let example = u32_at(off);
        let label = u32_at(off + 4) as f32;
        let x = data[off + 8..off + 8 + 4 * dim]
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        rows.push(Row { example, label, x });
        off += 8 + 4 * dim;
    }
    (rows, dim, layer)
}

/// Hidden states have wildly different scales per dimension, so standardize each one
/// with the training set's mean and standard deviation. The live script applies the same numbers.
fn standardize(rows: &mut [Row], train: &[bool], dim: usize) -> (Vec<f32>, Vec<f32>) {
    let n = train.iter().filter(|t| **t).count().max(1) as f32;
    let mut mean = vec![0.0f32; dim];
    let mut var = vec![0.0f32; dim];
    for (r, _) in rows.iter().zip(train).filter(|(_, t)| **t) {
        for i in 0..dim { mean[i] += r.x[i] / n; }
    }
    for (r, _) in rows.iter().zip(train).filter(|(_, t)| **t) {
        for i in 0..dim { var[i] += (r.x[i] - mean[i]).powi(2) / n; }
    }
    let std: Vec<f32> = var.iter().map(|v| v.sqrt().max(1e-4)).collect();
    for r in rows.iter_mut() {
        for i in 0..dim { r.x[i] = (r.x[i] - mean[i]) / std[i]; }
    }
    (mean, std)
}

fn shuffle<T>(v: &mut [T], seed: &mut u64) {
    for i in (1..v.len()).rev() {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        v.swap(i, (*seed >> 33) as usize % (i + 1));
    }
}

fn report(name: &str, (tp, fp, tn, fn_): (usize, usize, usize, usize)) {
    let total = (tp + fp + tn + fn_).max(1) as f32;
    let precision = tp as f32 / (tp + fp).max(1) as f32;
    let recall = tp as f32 / (tp + fn_).max(1) as f32;
    println!("{name}: accuracy {:.1}%, precision {:.1}%, recall {:.1}%  (tp {tp}, fp {fp}, tn {tn}, fn {fn_})",
        100.0 * (tp + tn) as f32 / total, 100.0 * precision, 100.0 * recall);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: probetrain <features.bin> [probe.bin]");
        std::process::exit(1);
    }
    let out = args.get(2).cloned().unwrap_or_else(|| "probe.bin".into());
    let (mut rows, dim, layer) = read_features(&args[1]);
    // hold out every 5th example (all of its tokens) so test rows come from unseen documents
    let train: Vec<bool> = rows.iter().map(|r| r.example % 5 != 0).collect();
    let (mean, std) = standardize(&mut rows, &train, dim);
    let mut train_rows: Vec<&Row> = rows.iter().zip(&train).filter(|(_, t)| **t).map(|(r, _)| r).collect();
    let test_rows: Vec<&Row> = rows.iter().zip(&train).filter(|(_, t)| !**t).map(|(r, _)| r).collect();
    println!("{} rows ({} train, {} test), dim {dim}, layer {layer}", rows.len(), train_rows.len(), test_rows.len());

    let mut w = vec![0.0f32; dim];
    let mut b = 0.0f32;
    let mut seed = 106;
    for epoch in 1..=15 {
        shuffle(&mut train_rows, &mut seed);
        let lr = 0.01 / (1.0 + epoch as f32 * 0.3);
        let loss = train_epoch(&mut w, &mut b, &train_rows, lr);
        println!("epoch {epoch:2}  loss {loss:.4}");
    }
    report("train", evaluate(&w, b, &train_rows));
    report("test ", evaluate(&w, b, &test_rows));

    let mut bytes = b"PRW1".to_vec();
    bytes.extend_from_slice(&(dim as u32).to_le_bytes());
    bytes.extend_from_slice(&layer.to_le_bytes());
    bytes.extend_from_slice(&b.to_le_bytes());
    for v in w.iter().chain(&mean).chain(&std) {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    fs::write(&out, bytes).expect("could not write probe file");
    println!("wrote {out}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_sigmoid_basics() {
        assert_eq!(sigmoid(0.0), 0.5);
        assert!(sigmoid(20.0) > 0.999_999 && sigmoid(-20.0) < 1e-6);
        assert!((sigmoid(2.0) + sigmoid(-2.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn probe_score_known_value() {
        // z = 0.5 + 2*1 + (-1)*1 = 1.5, sigmoid(1.5) = 0.817574
        assert!((score(&[2.0, -1.0], 0.5, &[1.0, 1.0]) - 0.817_574).abs() < 1e-5);
    }

    fn blobs(n: usize, seed: &mut u64) -> Vec<Row> {
        let mut noise = || {
            *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((*seed >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
        };
        (0..n)
            .map(|i| {
                let label = (i % 2) as f32;
                let c = if label == 1.0 { 1.5 } else { -1.5 };
                Row { example: i as u32, label, x: vec![c + noise(), c + noise(), noise()] }
            })
            .collect()
    }

    #[test]
    fn probe_learns_two_blobs() {
        let mut seed = 7;
        let train = blobs(200, &mut seed);
        let test = blobs(100, &mut seed);
        let (tr, te): (Vec<&Row>, Vec<&Row>) = (train.iter().collect(), test.iter().collect());
        let (mut w, mut b) = (vec![0.0; 3], 0.0);
        let first = train_epoch(&mut w, &mut b, &tr, 0.1);
        let mut last = first;
        for _ in 0..9 {
            last = train_epoch(&mut w, &mut b, &tr, 0.1);
        }
        assert!(last < first, "loss should go down: {first} -> {last}");
        let (tp, fp, tn, fn_) = evaluate(&w, b, &te);
        assert!((tp + tn) as f32 / 100.0 >= 0.95, "accuracy too low: tp {tp} fp {fp} tn {tn} fn {fn_}");
    }

    #[test]
    fn probe_confusion_counts() {
        // w = [1], b = 0: positive x scores above 0.5
        let rows = [
            Row { example: 0, label: 1.0, x: vec![2.0] },  // caught
            Row { example: 1, label: 0.0, x: vec![3.0] },  // false alarm
            Row { example: 2, label: 0.0, x: vec![-1.0] }, // correctly quiet
            Row { example: 3, label: 1.0, x: vec![-2.0] }, // missed
            Row { example: 4, label: 1.0, x: vec![0.5] },  // caught
        ];
        let refs: Vec<&Row> = rows.iter().collect();
        assert_eq!(evaluate(&[1.0], 0.0, &refs), (2, 1, 1, 1));
    }
}
