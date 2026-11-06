## Design

- For a primary card button, handle middle-button `auxclick` by opening only a parsed HTTP(S) URL in a new browsing context with `noopener,noreferrer`; do not call the visit/state endpoints or select the detail. State controls are separate and are not affected. Empty, malformed and other-scheme URLs do not open.
- Render a safe HTTP(S) original as the article title link in the existing detail; otherwise render a plain heading. Remove the redundant separate original link. No remote HTML is rendered; React escapes the article text.
- Synthetic browser tests intercept the original page, assert that initial and appended cards open without visit/read changes, validate safe new-window attributes and verify keyboard title activation. Test desktop/mobile and light/dark through the existing smoke suite; no real external site or old data is used.
