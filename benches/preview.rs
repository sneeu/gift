use criterion::{criterion_group, criterion_main, Criterion};

fn load_sample() -> Vec<u8> {
    let path = std::path::Path::new("benches/sample.gif");
    assert!(path.exists(), "Place a sample GIF at benches/sample.gif to run this benchmark");
    std::fs::read(path).expect("Failed to read benches/sample.gif")
}

fn bench_decode_gif(c: &mut Criterion) {
    let data = load_sample();
    c.bench_function("decode_gif", |b| {
        b.iter(|| gift::preview::decode_gif(criterion::black_box(&data)).unwrap())
    });
}

fn bench_png_encode(c: &mut Criterion) {
    let data = load_sample();
    let frames = gift::preview::decode_gif(&data).unwrap();
    let frame = &frames[0];

    c.bench_function("png_encode (1 frame)", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            criterion::black_box(frame)
                .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                .unwrap();
            buf
        })
    });
}

fn bench_png_encode_all(c: &mut Criterion) {
    let data = load_sample();
    let frames = gift::preview::decode_gif(&data).unwrap();
    eprintln!("frame count: {}", frames.len());

    c.bench_function("png_encode (all frames)", |b| {
        b.iter(|| {
            for frame in criterion::black_box(&frames) {
                let mut buf = Vec::new();
                frame
                    .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                    .unwrap();
            }
        })
    });
}

fn bench_png_decode(c: &mut Criterion) {
    let data = load_sample();
    let frames = gift::preview::decode_gif(&data).unwrap();
    let mut png_bytes = Vec::new();
    frames[0]
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .unwrap();

    c.bench_function("png_decode (1 frame)", |b| {
        b.iter(|| image::load_from_memory(criterion::black_box(&png_bytes)).unwrap())
    });
}

fn bench_png_decode_all(c: &mut Criterion) {
    let data = load_sample();
    let frames = gift::preview::decode_gif(&data).unwrap();
    let encoded: Vec<Vec<u8>> = frames
        .iter()
        .map(|f| {
            let mut buf = Vec::new();
            f.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                .unwrap();
            buf
        })
        .collect();

    c.bench_function("png_decode (all frames)", |b| {
        b.iter(|| {
            for png in criterion::black_box(&encoded) {
                image::load_from_memory(png).unwrap();
            }
        })
    });
}

criterion_group!(benches, bench_decode_gif, bench_png_encode, bench_png_encode_all, bench_png_decode, bench_png_decode_all);
criterion_main!(benches);
