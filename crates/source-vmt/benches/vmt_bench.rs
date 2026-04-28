use criterion::{black_box, criterion_group, criterion_main, Criterion, BatchSize};
use source_vmt::{Vmt, Value, intern_key};

const VMT_TEMPLATE: &str = r#"
    "VertexLitGeneric"
    {
        "$basetexture" "models/player/heavy/heavy_red"
        "$bumpmap"     "models/player/heavy/heavy_normal"
        "$phong"       "1"
        "$phongboost"  "1.5"
        "$phongexponent" "5"
        "$color"       "[1 1 1]"
        "Proxies"
        {
            "Sine"
            {
                "resultVar" "$color"
                "sineperiod" "2.0"
            }
        }
    }
"#;

fn bench_vmt_stress(c: &mut Criterion) {
    // 1. Single VMT Benchmarks (Baseline)
    let mut group = c.benchmark_group("vmt_core");
    
    group.bench_function("parse_single", |b| {
        b.iter(|| Vmt::from_str(black_box(VMT_TEMPLATE)).unwrap())
    });

    let vmt = Vmt::from_str(VMT_TEMPLATE).unwrap();
    group.bench_function("serialize_single", |b| {
        b.iter(|| black_box(&vmt).to_string().unwrap())
    });
    
    group.finish();

    // 2. Batch Parsing (100 VMTs)
    let mut group = c.benchmark_group("vmt_batch");
    let batch_input: Vec<String> = (0..100).map(|i| VMT_TEMPLATE.replace("heavy_red", &format!("heavy_red_{}", i))).collect();
    
    group.bench_function("parse_100_vmts", |b| {
        b.iter(|| {
            for input in &batch_input {
                let _ = Vmt::from_str(black_box(input)).unwrap();
            }
        })
    });

    // 3. Builder API Stress (Building from scratch)
    group.bench_function("build_from_scratch", |b| {
        b.iter(|| {
            let mut vmt = Vmt::new("VertexLitGeneric");
            vmt.set_string("$basetexture", "models/player/custom")
               .set_flag("$phong", true)
               .set_string("$phongboost", "2.5")
               .set_string("$phongexponent", "10")
               .add_proxy("Sine", [("resultVar", "$alpha"), ("sineperiod", "1.0")])
               .add_proxy("TextureScroll", [("texturescrollrate", "0.1")]);
            black_box(vmt);
        })
    });

    // 4. Bulk Lookups (50 lookups per iteration)
    let keys = vec![
        "basetexture", "bumpmap", "phong", "phongboost", "phongexponent", "color",
        "not_found_1", "not_found_2", "$basetexture", "%compilenodraw"
    ];
    group.bench_function("bulk_lookups_x50", |b| {
        b.iter(|| {
            for _ in 0..5 {
                for key in &keys {
                    black_box(vmt.get_raw(key));
                }
            }
        })
    });

    // 5. Patching Stress
    let patch = Vmt::from_str(r#"
        "patch"
        {
            "replace" { "$basetexture" "new/texture" }
            "insert" { "$rimlight" "1" }
        }
    "#).unwrap();

    group.bench_function("apply_patch", |b| {
        b.iter_batched(
            || vmt.clone(),
            |mut v| v.apply_patch(black_box(&patch)),
            BatchSize::SmallInput
        )
    });

    group.finish();
}

criterion_group!(benches, bench_vmt_stress);
criterion_main!(benches);
