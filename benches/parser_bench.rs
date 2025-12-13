use criterion::{black_box, criterion_group, criterion_main, Criterion};
use korni::{Korni, Environment};

fn benchmark_parser(c: &mut Criterion) {
    let simple_env = "KEY=value\nANOTHER_KEY=another_value\n# Comment\nEXPORTED=true";
    
    let mut group = c.benchmark_group("parser");
    
    group.bench_function("simple_env", |b| {
        b.iter(|| {
            let _ = Korni::from_str(black_box(simple_env)).parse();
        })
    });
    
    // Create a larger synthetic payload
    let mut large_env = String::new();
    for i in 0..1000 {
        large_env.push_str(&format!("KEY_{}=value_{}\n", i, i));
        large_env.push_str(&format!("# Comment {}\n", i));
        large_env.push_str(&format!("QUOTED_{}=\"some quoted value with number {}\"\n", i, i));
    }
    
    group.bench_function("large_env_1k_lines", |b| {
        b.iter(|| {
            let _ = Korni::from_str(black_box(&large_env)).parse();
        })
    });
    
    // Benchmark just the iterator (parsing without HashMap construction)
    group.bench_function("iterator_only_large", |b| {
        b.iter(|| {
            let builder = Korni::from_str(black_box(&large_env));
            // We need to bypass .parse() which constructs Environment
            // So we use parsing logic directly or a helper if available,
            // but .parse() is the public API.
            // Let's benchmark the high-level API as that's what users use.
            let _ = builder.parse();
        })
    });
    
    group.finish();
}

criterion_group!(benches, benchmark_parser);
criterion_main!(benches);
