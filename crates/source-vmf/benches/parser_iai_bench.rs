use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use vmf_forge::VmfFile;

// --- Benchmark Data ---
// Load VMF content at compile time using include_str!.
static VMF_CONTENT_SMALL: &str = include_str!("../vmf_examples/valid.vmf");
static VMF_CONTENT_LARGE: &str = include_str!("../vmf_examples/complex.vmf");
#[allow(dead_code)] // Used by 'parse_super_large_vmf', which is currently commented out below
static VMF_CONTENT_SUPER_LARGE: &str = include_str!("../vmf_examples/VERY_complex.vmf");

// --- Benchmark Functions ---

#[library_benchmark]
fn parse_small_vmf() -> VmfFile {
    // Using .expect() is acceptable here; if parsing fails, the benchmark
    // should panic, indicating a setup or code issue.
    // We return the result to prevent the compiler from optimizing away
    // the parsing logic entirely. While Callgrind inherently makes aggressive
    // optimizations less likely, returning the value is good practice.
    // `black_box` is usually not needed with iai-callgrind because Callgrind's
    // instrumentation itself prevents code removal, unlike timing-based benchmarks.
    VmfFile::parse(VMF_CONTENT_SMALL).expect("Benchmark failed: small VMF parsing error")
}

#[library_benchmark]
fn parse_super_large_vmf() -> VmfFile {
    // Same logic as parse_small_vmf, but with the larger dataset.
    VmfFile::parse(VMF_CONTENT_SUPER_LARGE)
        .expect("Benchmark failed: super large VMF parsing error")
}

#[library_benchmark]
fn parse_large_vmf() -> VmfFile {
    VmfFile::parse(VMF_CONTENT_LARGE).expect("Benchmark failed: large VMF parsing error")
}

// --- Benchmark Grouping and Main Entry Point ---

// Group the benchmarks together. This is optional but helps organize benchmarks
// logically, especially when you have many.
library_benchmark_group!(
    name = vmf_parsing_group;
    benchmarks = parse_small_vmf, parse_large_vmf //, parse_super_large_vmf
);

// Define the main entry point for the iai-callgrind benchmark runner.
main!(library_benchmark_groups = vmf_parsing_group);
