//! The two pages the loopback server serves to the browser: the sign-in page
//! that loads `MusicKit` JS and runs the authorization flow, and the success
//! page it lands on once the callback is captured.
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
