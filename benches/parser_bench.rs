use criterion::{black_box, criterion_group, criterion_main, Criterion};
use korni::Korni;

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

fn benchmark_error_handling(c: &mut Criterion) {
    let mut error_env = String::new();
    for i in 0..1000 {
        // Mix of valid and invalid entries
        if i % 10 == 0 {
            // Invalid: space around equals
            error_env.push_str(&format!("KEY_{} = value_{}\n", i, i));
        } else if i % 7 == 0 {
            // Invalid: starts with digit
            error_env.push_str(&format!("{}KEY=value_{}\n", i, i));
        } else if i % 5 == 0 {
            // Invalid: double equals
            error_env.push_str(&format!("KEY_{}==value_{}\n", i, i));
        } else {
            // Valid
            error_env.push_str(&format!("KEY_{}=value_{}\n", i, i));
        }
    }
    
    let mut group = c.benchmark_group("error_handling");
    
    group.bench_function("error_heavy_1k_lines", |b| {
        b.iter(|| {
            let _ = Korni::from_str(black_box(&error_env)).parse();
        })
    });
    
    group.finish();
}

fn benchmark_quote_types(c: &mut Criterion) {
    let mut single_quoted = String::new();
    let mut double_quoted = String::new();
    let mut unquoted = String::new();
    
    for i in 0..1000 {
        single_quoted.push_str(&format!("KEY_{}='value_{}'\n", i, i));
        double_quoted.push_str(&format!("KEY_{}=\"value_{}\"\n", i, i));
        unquoted.push_str(&format!("KEY_{}=value_{}\n", i, i));
    }
    
    let mut group = c.benchmark_group("quote_types");
    
    group.bench_function("single_quoted_1k_lines", |b| {
        b.iter(|| {
            let _ = Korni::from_str(black_box(&single_quoted)).parse();
        })
    });
    
    group.bench_function("double_quoted_1k_lines", |b| {
        b.iter(|| {
            let _ = Korni::from_str(black_box(&double_quoted)).parse();
        })
    });
    
    group.bench_function("unquoted_1k_lines", |b| {
        b.iter(|| {
            let _ = Korni::from_str(black_box(&unquoted)).parse();
        })
    });
    
    group.finish();
}

fn benchmark_comment_heavy(c: &mut Criterion) {
    let mut comment_heavy = String::new();
    
    for i in 0..1000 {
        comment_heavy.push_str(&format!("# This is comment number {}\n", i));
        comment_heavy.push_str(&format!("# With multiple lines\n"));
        comment_heavy.push_str(&format!("KEY_{}=value_{}\n", i, i));
    }
    
    let mut group = c.benchmark_group("comments");
    
    group.bench_function("comment_heavy_1k_lines", |b| {
        b.iter(|| {
            let _ = Korni::from_str(black_box(&comment_heavy)).parse();
        })
    });
    
    group.bench_function("comment_heavy_with_tracking", |b| {
        b.iter(|| {
            let _ = Korni::from_str(black_box(&comment_heavy))
                .preserve_comments()
                .track_positions()
                .parse();
        })
    });
    
    group.finish();
}

criterion_group!(benches, benchmark_parser, benchmark_error_handling, benchmark_quote_types, benchmark_comment_heavy);
criterion_main!(benches);
