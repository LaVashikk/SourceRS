use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::{fs, time::Duration};
use vmf_forge::VmfFile; // Adjust path if your library structure is different

/// Helper function to load VMF content from a file path.
/// Panics if the file cannot be read.
fn load_vmf_content(path: &str) -> String {
    fs::read_to_string(path).expect("Failed to read VMF file for benchmarking")
}

/// Defines the benchmark suite for VMF parsing.
fn benchmark_vmf_parsing(c: &mut Criterion) {
    // --- Setup: Load VMF files ---
    let vmf_content_small = load_vmf_content("vmf_examples/valid.vmf");
    let vmf_content_large = load_vmf_content("vmf_examples/complex.vmf");
    #[allow(unused_variables)] // Used by the "Parse Super Large VMF" benchmark below
    let vmf_content_very_large = load_vmf_content("vmf_examples/VERY_complex.vmf");

    // --- Benchmarks ---
    let mut group = c.benchmark_group("VMF Parsing");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(10));
    group.warm_up_time(Duration::from_secs(3));

    // Benchmark parsing a relatively small VMF file
    group.bench_function("Parse Small VMF", |b| {
        // `black_box` prevents the compiler optimizing away the function call
        b.iter(|| VmfFile::parse(black_box(&vmf_content_small)))
    });

    // Benchmark parsing a larger, more complex VMF file
    group.bench_function("Parse Large VMF", |b| {
        b.iter(|| VmfFile::parse(black_box(&vmf_content_large)))
    });

    // group.bench_function("Parse Super Large VMF", |b| {
    //     b.iter(|| VmfFile::parse(black_box(&vmf_content_very_large)))
    // });

    group.finish();
}

// Register the benchmark function(s). `benches` is the group name.
criterion_group!(benches, benchmark_vmf_parsing);
// Generate the main function that runs the benchmarks specified in the group.
criterion_main!(benches);
