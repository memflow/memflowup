const repo_path = "memflow/memflow-coredump";
const repo_branch = "master";

fn build_from_git() {
    let archive_url = "https://api.github.com/repos/" + repo_path + "/zipball/" + repo_branch;
    download_zip(archive_url, tmp_folder("memflow-coredump"));
}
