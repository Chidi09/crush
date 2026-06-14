use tracing::info;
use crush_build::StackDetector;
use crush_image::ImageStore;
use crate::BuildArgs;

pub async fn exec(
    args: &BuildArgs,
    store: &ImageStore,
) -> anyhow::Result<()> {
    if let Some(ref plat) = args.platform {
        std::env::set_var("CRUSH_DEFAULT_PLATFORM", plat);
    }
    info!("Building image: {} (platforms: {:?})", args.tag, args.platform);
    let detector = StackDetector::new();
    let project_root = std::env::current_dir()?;
    let stack = detector.detect(&project_root).await?;

    let base_image = stack.base_image.clone();
    println!("↳ stack: {} · base image: {}", stack.language, base_image);

    // The project is built natively (the dev loop already produced node_modules /
    // dist / a compiled binary). Package that result on top of the base image as
    // a real, registered, runnable OCI image — no Dockerfile, no daemon.
    let workdir = "/app";
    let entry = if !stack.entry_point.trim().is_empty() {
        stack.entry_point.clone()
    } else {
        stack.build_command.clone()
    };
    let cmd = if entry.trim().is_empty() {
        Vec::new() // fall back to the base image's default CMD
    } else {
        vec!["sh".to_string(), "-c".to_string(), entry]
    };
    let env = vec![format!("PORT={}", stack.default_port)];

    println!("↳ pulling base + packaging project (this may pull the base image once)...");
    let image = store
        .commit_app_image(&args.tag, &base_image, &project_root, workdir, cmd, env)
        .await?;

    println!("✓ crushed {} → {}", args.tag, &image.digest[..image.digest.len().min(19)]);
    println!("  layers: {} · size: {:.1} MB", image.layers.len(), image.size_bytes as f64 / 1_000_000.0);
    println!("  it's now in `crush images` — run it, or `crush export {} -o app.tar` then `docker load -i app.tar`.", args.tag);
    Ok(())
}
