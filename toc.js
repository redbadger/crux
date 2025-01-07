// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="overview.html">Overview</a></li><li class="chapter-item expanded affix "><a href="motivation.html">Motivation</a></li><li class="chapter-item expanded affix "><li class="part-title">Getting Started</li><li class="chapter-item expanded "><a href="getting_started/core.html"><strong aria-hidden="true">1.</strong> Shared core and types</a></li><li class="chapter-item expanded "><a href="getting_started/iOS/index.html"><strong aria-hidden="true">2.</strong> iOS</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="getting_started/iOS/with_xcodegen.html"><strong aria-hidden="true">2.1.</strong> Swift and SwiftUI (XcodeGen)</a></li><li class="chapter-item expanded "><a href="getting_started/iOS/manual.html"><strong aria-hidden="true">2.2.</strong> Swift and SwiftUI (manual)</a></li></ol></li><li class="chapter-item expanded "><a href="getting_started/Android/index.html"><strong aria-hidden="true">3.</strong> Android</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="getting_started/Android/android.html"><strong aria-hidden="true">3.1.</strong> Kotlin and Jetpack Compose</a></li></ol></li><li class="chapter-item expanded "><a href="getting_started/web/index.html"><strong aria-hidden="true">4.</strong> Web</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="getting_started/web/nextjs.html"><strong aria-hidden="true">4.1.</strong> TypeScript and React (Next.js)</a></li><li class="chapter-item expanded "><a href="getting_started/web/remix.html"><strong aria-hidden="true">4.2.</strong> TypeScript and React (Remix)</a></li><li class="chapter-item expanded "><a href="getting_started/web/svelte.html"><strong aria-hidden="true">4.3.</strong> TypeScript and Svelte (Parcel)</a></li><li class="chapter-item expanded "><a href="getting_started/web/yew.html"><strong aria-hidden="true">4.4.</strong> Rust and Yew</a></li><li class="chapter-item expanded "><a href="getting_started/web/leptos.html"><strong aria-hidden="true">4.5.</strong> Rust and Leptos</a></li><li class="chapter-item expanded "><a href="getting_started/web/dioxus.html"><strong aria-hidden="true">4.6.</strong> Rust and Dioxus</a></li></ol></li><li class="chapter-item expanded "><li class="part-title">Development Guide</li><li class="chapter-item expanded "><a href="guide/hello_world.html"><strong aria-hidden="true">5.</strong> Hello world</a></li><li class="chapter-item expanded "><a href="guide/elm_architecture.html"><strong aria-hidden="true">6.</strong> Elm Architecture</a></li><li class="chapter-item expanded "><a href="guide/capabilities.html"><strong aria-hidden="true">7.</strong> Capabilities</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="guide/capability_apis.html"><strong aria-hidden="true">7.1.</strong> Capability APIs</a></li></ol></li><li class="chapter-item expanded "><a href="guide/testing.html"><strong aria-hidden="true">8.</strong> Testing</a></li><li class="chapter-item expanded "><a href="guide/message_interface.html"><strong aria-hidden="true">9.</strong> Interface between core and shell</a></li><li class="chapter-item expanded "><a href="guide/composing.html"><strong aria-hidden="true">10.</strong> Composable Applications</a></li><li class="chapter-item expanded affix "><li class="part-title">Internals</li><li class="chapter-item expanded "><a href="internals/runtime.html"><strong aria-hidden="true">11.</strong> Capability runtime and Effects</a></li><li class="chapter-item expanded "><a href="internals/bridge.html"><strong aria-hidden="true">12.</strong> FFI bridge</a></li><li class="chapter-item expanded "><a href="internals/effect.html"><strong aria-hidden="true">13.</strong> The Effect type</a></li><li class="chapter-item expanded "><a href="internals/typegen.html"><strong aria-hidden="true">14.</strong> Type generation</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString();
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
