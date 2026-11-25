// ch10cal: calibrate the confidence channel against questions with known answers.
// Milestone 7 (Calibration Caverns). Session plan: SESSION-milestone7.md
//
// usage: ch10cal <calib_results.csv> [report.txt]
//
// Every instrument gets a calibration and an error band. A strain gauge maps volts to
// microstrain. Here the question is: when the entropy channel reads high, how often
// is the model actually wrong? This program answers that from real graded answers.

#![allow(unused_variables, dead_code)] // delete this line once all YOUR PARTs are done

use std::env;
use std::fs;

/// One bin of the reliability table.
#[derive(Debug, PartialEq)]
struct Bin {
    lo: f64,
    hi: f64,
    n: usize,
    errors: usize,
}

/// How well a score separates wrong answers from right ones.
/// 1.0 = every wrong answer scored higher than every right one, 0.5 = no better than a coin.
fn auroc(scores: &[f64], wrong: &[bool]) -> f64 {
    // YOUR PART 16: compare every (wrong, right) pair.
    // 1. for each wrong answer w and each right answer r:
    //      score[w] > score[r]  counts 1
    //      score[w] == score[r] counts 0.5
    //      otherwise            counts 0
    // 2. divide the total by (number of wrong) * (number of right)
    // 3. if there are no wrong answers or no right ones, return 0.5 (nothing to separate)
    //
    // Done when: cargo test --bin ch10cal cal_auroc
    todo!("auroc")
}

/// Split the score range into equal-width bins and count answers and errors in each.
fn reliability(scores: &[f64], wrong: &[bool], bins: usize) -> Vec<Bin> {
    // YOUR PART 17: a histogram that also counts errors.
    // 1. lo = smallest score, hi = largest score, width = (hi - lo) / bins
    // 2. make `bins` Bins: bin k covers lo + k*width .. lo + (k+1)*width
    // 3. for each answer: k = ((score - lo) / width) as usize, but
    //      - the largest score belongs in the LAST bin (clamp k to bins - 1)
    //      - if width is 0 (every score the same), everything goes in bin 0
    //    then n += 1, and errors += 1 if it was wrong
    //
    // Done when: cargo test --bin ch10cal cal_reliability
    todo!("reliability")
}

/// 95% confidence interval for an error rate (Wilson score interval).
/// Small bins get wide bands, which is honest.
fn wilson(errors: usize, n: usize) -> (f64, f64) {
    // YOUR PART 18: the error band.
    // 1. if n == 0, return (0.0, 1.0): no data, could be anything
    // 2. p = errors / n, z = 1.96
    //    denom  = 1 + z*z/n
    //    center = (p + z*z/(2n)) / denom
    //    half   = z * sqrt(p*(1-p)/n + z*z/(4n*n)) / denom
    // 3. return (center - half, center + half), each clamped to 0.0..=1.0
    //
    // Done when: cargo test --bin ch10cal cal_wilson
    todo!("wilson")
}

/// Pre-filled: read the graded results the Python script wrote.
fn read_results(path: &str) -> (Vec<String>, Vec<Vec<f64>>, Vec<bool>) {
    let features = ["mean_entropy", "max_entropy", "first_entropy", "mean_top_prob"];
    let mut reader = csv::Reader::from_path(path).expect("could not open results CSV");
    let headers = reader.headers().expect("no header row").clone();
    let col = |name: &str| headers.iter().position(|h| h == name).unwrap_or_else(|| panic!("missing column {name}"));
    let fcols: Vec<usize> = features.iter().map(|f| col(f)).collect();
    let correct = col("correct");
    let mut values = vec![Vec::new(); features.len()];
    let mut wrong = Vec::new();
    for rec in reader.records() {
        let row = rec.expect("bad CSV row");
        for (k, &c) in fcols.iter().enumerate() {
            let v: f64 = row[c].parse().expect("bad number");
            // top probability is high when the model is sure, so flip it into an "unsureness" score
            values[k].push(if features[k] == "mean_top_prob" { 1.0 - v } else { v });
        }
        wrong.push(&row[correct] != "1");
    }
    let names = vec!["mean entropy".into(), "max entropy".into(), "first-token entropy".into(), "1 - mean top prob".into()];
    (names, values, wrong)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: ch10cal <calib_results.csv> [report.txt]");
        std::process::exit(1);
    }
    let out_path = args.get(2).cloned().unwrap_or_else(|| "calibration.txt".into());
    let (names, values, wrong) = read_results(&args[1]);
    let n = wrong.len();
    let n_wrong = wrong.iter().filter(|w| **w).count();

    let mut report = String::new();
    report += &format!("Calibration from {} ({} answers, {} wrong, {:.0}% error rate)\n\n", args[1], n, n_wrong,
        100.0 * n_wrong as f64 / n.max(1) as f64);
    report += "How well each channel separates wrong answers from right ones (AUROC, 0.5 = coin flip):\n";
    let mut best = 0;
    for (k, name) in names.iter().enumerate() {
        let a = auroc(&values[k], &wrong);
        report += &format!("  {:22} {:.3}\n", name, a);
        if a > auroc(&values[best], &wrong) {
            best = k;
        }
    }
    let best_auroc = auroc(&values[best], &wrong);
    report += &format!("\nbest AUROC {:.3} ({})\n\nReliability for {}, with 95% error bands:\n", best_auroc, names[best], names[best]);
    report += "  score range        answers  error rate   95% band\n";
    for b in reliability(&values[best], &wrong, 5) {
        let (lo, hi) = wilson(b.errors, b.n);
        let rate = if b.n > 0 { format!("{:5.1}%", 100.0 * b.errors as f64 / b.n as f64) } else { "   -  ".into() };
        report += &format!("  {:6.3} to {:6.3}   {:7}  {:>9}   {:4.0}% to {:3.0}%\n", b.lo, b.hi, b.n, rate, lo * 100.0, hi * 100.0);
    }
    print!("{report}");
    fs::write(&out_path, &report).expect("could not write report");
    eprintln!("\nwrote {out_path}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cal_auroc_textbook_example() {
        // wrong answers scored 0.35 and 0.8; right ones 0.1 and 0.4 -> 3 of 4 pairs ordered
        let a = auroc(&[0.1, 0.4, 0.35, 0.8], &[false, false, true, true]);
        assert!((a - 0.75).abs() < 1e-12);
    }

    #[test]
    fn cal_auroc_extremes_and_ties() {
        assert_eq!(auroc(&[0.0, 1.0], &[false, true]), 1.0);
        assert_eq!(auroc(&[1.0, 0.0], &[false, true]), 0.0);
        assert_eq!(auroc(&[0.5, 0.5, 0.5], &[false, true, true]), 0.5);
        assert_eq!(auroc(&[0.1, 0.9], &[false, false]), 0.5); // nothing wrong to separate
    }

    #[test]
    fn cal_reliability_bins_scores() {
        let bins = reliability(&[0.0, 0.1, 0.5, 0.9, 1.0], &[false, false, true, true, true], 2);
        assert_eq!(bins.len(), 2);
        assert_eq!((bins[0].n, bins[0].errors), (2, 0));
        assert_eq!((bins[1].n, bins[1].errors), (3, 3)); // 1.0 lands in the last bin
        assert!((bins[1].lo - 0.5).abs() < 1e-12 && (bins[1].hi - 1.0).abs() < 1e-12);
    }

    #[test]
    fn cal_reliability_handles_identical_scores() {
        let bins = reliability(&[0.3, 0.3, 0.3], &[true, false, false], 4);
        assert_eq!((bins[0].n, bins[0].errors), (3, 1));
        assert_eq!(bins.iter().map(|b| b.n).sum::<usize>(), 3);
    }

    #[test]
    fn cal_wilson_matches_reference() {
        let (lo, hi) = wilson(5, 20);
        assert!((lo - 0.111_860).abs() < 1e-5 && (hi - 0.468_705).abs() < 1e-5);
        let (lo, hi) = wilson(0, 10);
        assert!(lo == 0.0 && (hi - 0.277_540).abs() < 1e-5);
        assert_eq!(wilson(0, 0), (0.0, 1.0));
    }
}
