use criterion::{Criterion, criterion_group, criterion_main};
use std::ffi::OsString;
use std::hint::black_box;
use std::path::PathBuf;
use xtask::{
    CommandFamily, OutputFormat, XtaskCommand, XtaskEnvironment, parse_xtask_command, route_command,
};

fn benchmark_parse_required_command(c: &mut Criterion) {
    c.bench_function("vb_kkvb_parse_ai_context", |bench| {
        bench.iter(|| {
            parse_xtask_command(black_box([
                OsString::from("xtask"),
                OsString::from("ai-context"),
                OsString::from("--bead"),
                OsString::from("vb-kkvb"),
                OsString::from("--format"),
                OsString::from("jsonl"),
            ]))
        })
    });
}

fn benchmark_route_deferred_command(c: &mut Criterion) {
    let env = XtaskEnvironment {
        workspace_root: PathBuf::from("/workspace"),
        bead_id: Some("vb-kkvb".to_string()),
        output_format: OutputFormat::JsonLines,
        unavailable_families: Vec::new(),
    };
    let command = XtaskCommand::Required(CommandFamily::AiContext);

    c.bench_function("vb_kkvb_route_ai_context_deferred", |bench| {
        bench.iter(|| route_command(black_box(command.clone()), black_box(&env)))
    });
}

criterion_group!(
    benches,
    benchmark_parse_required_command,
    benchmark_route_deferred_command
);
criterion_main!(benches);
