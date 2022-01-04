// build from source
fn build() {
    /*
    info("downloading from " + repo_path);
    info("downloading to " + tmp_dir());

    let archive_url = "https://api.github.com/repos/" + repo_path + "/tarball/" + repo_branch;
    let archive_path = tmp_file("memflow-coredump.tar.gz");
    if download(archive_url, archive_path) {
        info("github archive downloaded");
    } else {
        error("error downloading archive from github");
    }
    */

    download_repository("memflow", "memflow-pcileech");
}

// installs the given connector to user directory
fn install() {
    //install_connector(out_folder + "/target/release/libmemflow_coredump.so");
}
