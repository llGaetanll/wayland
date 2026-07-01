// Firefox prefs managed from dotfiles (~/.config/firefox/user.js), symlinked
// into the active profile. user.js is re-applied on every Firefox start.

// Enable userChrome.css / userContent.css (needed for the traffic-light title bar).
user_pref("toolkit.legacyUserProfileCustomizations.stylesheets", true);
