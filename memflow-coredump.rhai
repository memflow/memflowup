const repo_path = "memflow/memflow-coredump";
const repo_branch = "master";
const out_folder = tmp_folder("memflow-coredump");

// builds the connector from source (github)
fn build() {
    // TODO: clean output folder if it exists
    //let archive_url = "https://api.github.com/repos/" + repo_path + "/zipball/" + repo_branch;
    //download_zip(archive_url, out_folder, 1);
    //cargo("build --release --all-features", out_folder);


    download_repository("memflow", "memflow-coredump");
}

// installs the given connector to user directory
fn install() {
    //install_connector(out_folder + "/target/release/libmemflow_coredump.so");
}
