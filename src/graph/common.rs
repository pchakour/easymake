use url::Url;

pub fn is_downloadable_file(url: &str) -> bool {
    // Parse the URL
    if let Ok(parsed_url) = Url::parse(url) {
        // Check if the scheme is HTTP or HTTPS
        if parsed_url.scheme() == "http" || parsed_url.scheme() == "https" {
            // Extract the path and check if it looks like a file
            if let Some(path) = parsed_url.path_segments() {
                if let Some(last_segment) = path.last() {
                    return !last_segment.contains(".git")  // Check if not a git repository
                }
            }
        }
    }
    false
}

