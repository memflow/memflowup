
// builds the connector from a given path (never called by the engine directly)
fn build_from_path(ctx, repo_path) {
    cargo("build --release --all-features", repo_path);
	ctx.copy_cargo_plugin_artifact(repo_path, name_to_lib(ctx.crate_name()));
}

// builds the connector from local path
fn build_local(ctx) {
	build_from_path(ctx, ctx.build_path())
}

// builds the connector from source (github)
fn build_from_source(ctx) {
	build_from_path(ctx, get_source(ctx))
}

// downloads a binary release of a plugin
fn install(ctx) {
	let release = ctx.github_latest_release();
	let libname = name_to_lib_with_arch(ctx.crate_name());
	let artifact = download(ctx.github_release_artifact(release, libname));
	write_plugin_artifact(artifact);
}
