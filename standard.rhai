
// builds the connector from source (github)
fn build_from_source(ctx) {
	let repo_path = ctx.extract(ctx.download_repository());
    cargo("build --release --all-features", repo_path);
	ctx.copy_cargo_plugin_artifact(repo_path, name_to_lib(ctx.crate_name()));
}

// downloads a binary release of a plugin
fn install(ctx) {
	let release = ctx.github_latest_release();
	let libname = name_to_lib_with_arch(ctx.crate_name());
	let artifact = download(ctx.github_release_artifact(release, libname));
	write_plugin_artifact(artifact);
}
