//! The pages the sign-in flow puts in front of the browser: the sign-in page
//! that loads `MusicKit` JS and runs the authorization flow, the success page
//! it lands on once the callback is captured, and the local bootstrap page that
//! redirects the browser to the loopback sign-in URL.
//!
//! The page is a static template; [`render_auth_page`] substitutes the
//! developer token and the per-flow `state` nonce into it. Keeping the HTML
//! here leaves the parent module's flow and protocol code free of embedded
//! markup.

/// Builds the sign-in page, substituting the developer token, nonce, and build
/// version.
pub(super) fn render_auth_page(developer_token: &str, nonce: &str) -> String {
    AUTH_PAGE
        .replace("{{DEVELOPER_TOKEN}}", developer_token)
        .replace("{{STATE}}", nonce)
        .replace("{{VERSION}}", env!("CARGO_PKG_VERSION"))
}

/// Builds the local bootstrap page that redirects the browser to the sign-in
/// URL.
///
/// The flow writes this page into an owner-only temp file and opens that file,
/// rather than passing the URL to the browser: the URL carries the per-flow
/// `state` nonce, and a process's command line is world-readable on Unix. `url`
/// is assembled from a numeric port and a hex nonce, so it contains no HTML
/// metacharacters and needs no escaping.
pub(super) fn render_bootstrap_page(url: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta http-equiv=\"refresh\" content=\"0; url={url}\">\n<title>Nostalgia - Apple Music sign-in</title>\n</head>\n<body>\n<p>Opening the Apple Music sign-in page...</p>\n</body>\n</html>\n"
    )
}

/// The page opened in the browser: it loads `MusicKit` JS, configures it, calls
/// `authorize()`, and posts the resulting user token back to `/token`.
const AUTH_PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Nostalgia - Apple Music sign-in</title>
</head>
<body>
<h1>Nostalgia - Apple Music sign-in</h1>
<p id="status">Loading MusicKit...</p>
<script src="https://js-cdn.music.apple.com/musickit/v3/musickit.js"></script>
<script>
var developerToken = "{{DEVELOPER_TOKEN}}";
var state = "{{STATE}}";
document.addEventListener("musickitloaded", function () {
  MusicKit.configure({
    developerToken: developerToken,
    app: { name: "Nostalgia", build: "{{VERSION}}" }
  });
  MusicKit.getInstance().authorize().then(function (userToken) {
    var body = "state=" + encodeURIComponent(state) +
      "&userToken=" + encodeURIComponent(userToken);
    return fetch("/token", {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: body
    });
  }).then(function () {
    document.getElementById("status").textContent =
      "Sign-in complete. You can close this tab.";
  }).catch(function (error) {
    document.getElementById("status").textContent = "Sign-in failed: " + error;
  });
});
</script>
</body>
</html>
"#;

/// The page the browser lands on after a successful callback.
pub(super) const SUCCESS_PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Nostalgia - signed in</title>
</head>
<body>
<h1>Signed in to Apple Music</h1>
<p>You can close this tab and return to Nostalgia.</p>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::{SUCCESS_PAGE, render_auth_page, render_bootstrap_page};

    // The bootstrap page exists to redirect the browser to the loopback
    // sign-in URL, which the flow cannot pass on the command line (the URL
    // carries the per-flow nonce, and argv is world-readable).
    // `bootstrap_page_is_private_and_carries_the_sign_in_url` pins the file
    // mechanics and that the URL appears in the page, but not the redirect:
    // leaving the URL as plain body text would keep that test green while the
    // browser sat on the bootstrap page until the flow timed out. Pin the meta
    // refresh that actually performs the redirect.
    #[test]
    fn bootstrap_page_meta_refreshes_to_the_sign_in_url() {
        let url = "http://127.0.0.1:4321/?state=deadbeefdeadbeef";
        let page = render_bootstrap_page(url);

        assert!(
            page.contains(&format!(
                "<meta http-equiv=\"refresh\" content=\"0; url={url}\">"
            )),
            "the bootstrap page must meta-refresh to the sign-in URL, got: {page}"
        );
    }

    // The sign-in page only works if the browser loads MusicKit JS and calls
    // `authorize()`. `render_auth_page_substitutes_every_template_placeholder`
    // pins the token, nonce, and version substitutions, but a page that
    // dropped the script tag (or the `authorize()` call) would keep every
    // placeholder assertion green while no user token was ever obtained. Pin
    // the two things that make the page do its job.
    #[test]
    fn auth_page_loads_musickit_and_calls_authorize() {
        let page = render_auth_page("dev-token", "deadbeefdeadbeef");

        assert!(
            page.contains("musickit.js"),
            "the sign-in page must load MusicKit JS, got: {page}"
        );
        assert!(
            page.contains(".authorize()"),
            "the sign-in page must call MusicKit's authorize(), got: {page}"
        );
    }

    // The callback serves `SUCCESS_PAGE` after storing the session. The flow
    // test pins the 200 status but never the body, so a regression that served
    // a blank or wrong page would clear the suite while the user's browser
    // showed nothing. Pin that it announces the completed sign-in and does not
    // re-load the sign-in script (which would restart the flow).
    #[test]
    fn success_page_announces_the_sign_in_without_restarting_it() {
        assert!(
            SUCCESS_PAGE.contains("Signed in to Apple Music"),
            "the success page must tell the user the sign-in completed, got: {SUCCESS_PAGE}"
        );
        assert!(
            !SUCCESS_PAGE.contains("musickit.js"),
            "the success page must not re-load the sign-in flow, got: {SUCCESS_PAGE}"
        );
    }
}
