/*
 * feed-me admin — Alpine app
 *
 * Single root component (`adminApp`) with a tiny hash router:
 *   #/feeds            feed list
 *   #/feeds/:id        feed detail + entries
 *   #/users            user list
 *   #/login            login screen (also shown on any 401)
 */
document.addEventListener("alpine:init", () => {
    Alpine.data("adminApp", () => ({
        route: { name: "feeds", params: {} },
        toasts: [],

        // login
        loginData: { username: "", password: "" },
        loginError: "",
        loginBusy: false,

        // feeds
        feeds: [],
        feedsLoading: false,
        feedForm: { open: false, mode: "create", id: null, busy: false, data: {} },

        // feed detail
        currentFeed: null,
        entries: [],
        entriesLoading: false,
        entryFilter: "all",
        entryForm: { open: false, mode: "create", id: null, busy: false, data: {} },

        // users
        users: [],
        usersLoading: false,
        userForm: { open: false, mode: "create", id: null, busy: false, data: {} },

        // ── lifecycle ───────────────────────────────────────
        init() {
            window.addEventListener("unauthorized", () => this.onUnauthorized());
            window.addEventListener("hashchange", () => this.handleRoute());
            this.handleRoute();
        },

        onUnauthorized() {
            if (this.route.name !== "login") {
                this.loginError = "Please sign in.";
                window.location.hash = "#/login";
            }
        },

        // ── routing ─────────────────────────────────────────
        async handleRoute() {
            const path = (window.location.hash.replace(/^#/, "") || "/feeds");
            const parts = path.split("/").filter(Boolean);

            if (parts[0] === "login") {
                this.route = { name: "login", params: {} };
            } else if (parts[0] === "users") {
                this.route = { name: "users", params: {} };
                await this.loadUsers();
            } else if (parts[0] === "feeds" && parts[1]) {
                this.route = { name: "feed", params: { id: parts[1] } };
                await this.loadFeed(parts[1]);
            } else {
                this.route = { name: "feeds", params: {} };
                await this.loadFeeds();
            }
        },

        navigate(hash) {
            if (window.location.hash === hash) this.handleRoute();
            else window.location.hash = hash;
        },

        // ── toasts / errors ─────────────────────────────────
        notify(msg, type = "error") {
            const id = Date.now() + Math.random();
            this.toasts.push({ id, msg, type });
            setTimeout(() => {
                this.toasts = this.toasts.filter((t) => t.id !== id);
            }, 5000);
        },

        errText(e) {
            if (e instanceof api.ApiError) {
                if (e.status === 404) return "Not available yet (backend endpoint missing).";
                if (e.body && typeof e.body === "object") {
                    return e.body.message || e.body.error || JSON.stringify(e.body);
                }
                return e.message;
            }
            return e && e.message ? e.message : String(e);
        },

        handle(e) {
            if (e instanceof api.ApiError && e.status === 401) return; // handled globally
            this.notify(this.errText(e));
        },

        // ── auth ────────────────────────────────────────────
        async doLogin() {
            this.loginBusy = true;
            this.loginError = "";
            try {
                await api.login(this.loginData.username, this.loginData.password);
                this.loginData.password = "";
                this.navigate("#/feeds");
            } catch (e) {
                this.loginError = e instanceof api.ApiError && e.status === 401
                    ? "Invalid username or password."
                    : this.errText(e);
            } finally {
                this.loginBusy = false;
            }
        },

        async doLogout() {
            try { await api.logout(); } catch (e) { /* ignore */ }
            window.location.hash = "#/login";
        },

        // ── feeds ───────────────────────────────────────────
        async loadFeeds() {
            this.feedsLoading = true;
            try {
                this.feeds = (await api.feeds.list()) || [];
            } catch (e) {
                this.handle(e);
            } finally {
                this.feedsLoading = false;
            }
        },

        async loadFeed(id) {
            this.currentFeed = null;
            this.entries = [];
            try {
                this.currentFeed = await api.feeds.get(id);
                await this.loadEntries();
            } catch (e) {
                this.handle(e);
            }
        },

        openFeedForm(mode, feed) {
            this.feedForm = {
                open: true,
                mode,
                id: feed ? feed.id : null,
                busy: false,
                data: feed
                    ? { name: feed.name, feed_type: feed.feed_type, description: feed.description || "" }
                    : { name: "", feed_type: "ip", description: "" },
            };
        },

        async submitFeedForm() {
            this.feedForm.busy = true;
            try {
                if (this.feedForm.mode === "create") {
                    await api.feeds.create({
                        name: this.feedForm.data.name,
                        feed_type: this.feedForm.data.feed_type,
                        description: this.feedForm.data.description || null,
                    });
                    this.notify("Feed created.", "success");
                } else {
                    await api.feeds.update(this.feedForm.id, {
                        description: this.feedForm.data.description || null,
                    });
                    this.notify("Feed updated.", "success");
                }
                this.feedForm.open = false;
                if (this.route.name === "feed") await this.loadFeed(this.route.params.id);
                else await this.loadFeeds();
            } catch (e) {
                this.handle(e);
            } finally {
                this.feedForm.busy = false;
            }
        },

        async deleteFeed(feed) {
            if (!confirm(`Delete feed "${feed.name}" and all its entries?`)) return;
            try {
                await api.feeds.remove(feed.id);
                this.notify("Feed deleted.", "success");
                if (this.route.name === "feed") this.navigate("#/feeds");
                else await this.loadFeeds();
            } catch (e) {
                this.handle(e);
            }
        },

        // ── entries ─────────────────────────────────────────
        async loadEntries() {
            if (!this.currentFeed) return;
            this.entriesLoading = true;
            const params = {};
            if (this.entryFilter === "enabled") params.enabled = "true";
            if (this.entryFilter === "disabled") params.enabled = "false";
            try {
                this.entries = (await api.entries.list(this.currentFeed.id, params)) || [];
            } catch (e) {
                this.handle(e);
            } finally {
                this.entriesLoading = false;
            }
        },

        openEntryForm(mode, entry) {
            this.entryForm = {
                open: true,
                mode,
                id: entry ? entry.id : null,
                busy: false,
                data: entry
                    ? {
                          value: entry.value,
                          description: entry.description || "",
                          enabled: entry.enabled,
                          valid_until: toLocalInput(entry.valid_until),
                      }
                    : { value: "", description: "", enabled: true, valid_until: "" },
            };
        },

        valuePlaceholder() {
            const t = this.currentFeed && this.currentFeed.feed_type;
            if (t === "ip") return "203.0.113.0/24 or 198.51.100.7";
            if (t === "domain") return "malicious.example.com";
            if (t === "url") return "https://example.com/bad/path";
            return "";
        },

        async submitEntryForm() {
            this.entryForm.busy = true;
            const payload = {
                description: this.entryForm.data.description || null,
                enabled: !!this.entryForm.data.enabled,
                valid_until: fromLocalInput(this.entryForm.data.valid_until),
            };
            try {
                if (this.entryForm.mode === "create") {
                    payload.value = this.entryForm.data.value;
                    await api.entries.create(this.currentFeed.id, payload);
                    this.notify("Entry added.", "success");
                } else {
                    await api.entries.update(this.currentFeed.id, this.entryForm.id, payload);
                    this.notify("Entry updated.", "success");
                }
                this.entryForm.open = false;
                await this.loadEntries();
            } catch (e) {
                this.handle(e);
            } finally {
                this.entryForm.busy = false;
            }
        },

        async toggleEntry(entry, enabled) {
            try {
                await api.entries.update(this.currentFeed.id, entry.id, {
                    enabled,
                    description: entry.description || null,
                    valid_until: entry.valid_until || null,
                });
                entry.enabled = enabled;
            } catch (e) {
                this.handle(e);
                await this.loadEntries(); // resync the switch
            }
        },

        async deleteEntry(entry) {
            if (!confirm(`Delete entry "${entry.value}"?`)) return;
            try {
                await api.entries.remove(this.currentFeed.id, entry.id);
                this.notify("Entry deleted.", "success");
                await this.loadEntries();
            } catch (e) {
                this.handle(e);
            }
        },

        // ── users ───────────────────────────────────────────
        async loadUsers() {
            this.usersLoading = true;
            try {
                this.users = (await api.users.list()) || [];
            } catch (e) {
                this.handle(e);
            } finally {
                this.usersLoading = false;
            }
        },

        openUserForm(mode, user) {
            this.userForm = {
                open: true,
                mode,
                id: user ? user.id : null,
                busy: false,
                data: user
                    ? { name: user.name, email: user.email, password: "" }
                    : { name: "", email: "", password: "" },
            };
        },

        async submitUserForm() {
            this.userForm.busy = true;
            try {
                if (this.userForm.mode === "create") {
                    await api.users.create({
                        name: this.userForm.data.name,
                        email: this.userForm.data.email,
                        password: this.userForm.data.password,
                    });
                    this.notify("User created.", "success");
                } else {
                    await api.users.update(this.userForm.id, {
                        name: this.userForm.data.name,
                        email: this.userForm.data.email,
                    });
                    this.notify("User updated.", "success");
                }
                this.userForm.open = false;
                await this.loadUsers();
            } catch (e) {
                this.handle(e);
            } finally {
                this.userForm.busy = false;
            }
        },

        async deleteUser(user) {
            if (!confirm(`Delete user "${user.name}"?`)) return;
            try {
                await api.users.remove(user.id);
                this.notify("User deleted.", "success");
                await this.loadUsers();
            } catch (e) {
                this.handle(e);
            }
        },

        // ── view helpers ────────────────────────────────────
        feedUrl(feed) {
            return `${window.location.origin}/feed/${feed.name}`;
        },

        async copyFeedUrl(feed) {
            const url = this.feedUrl(feed);
            try {
                await navigator.clipboard.writeText(url);
                this.notify("Feed URL copied.", "success");
            } catch {
                this.notify(url, "success");
            }
        },

        shortDigest(digest) {
            const hex = toHex(digest);
            return hex ? hex.slice(0, 12) + "…" : "—";
        },

        fmtDate(value) {
            if (!value) return "—";
            const d = new Date(value);
            return isNaN(d) ? value : d.toLocaleString();
        },
    }));
});

// ── module-scope helpers ────────────────────────────────────
function toHex(digest) {
    if (!digest) return "";
    if (typeof digest === "string") return digest;
    if (Array.isArray(digest)) {
        return digest.map((b) => b.toString(16).padStart(2, "0")).join("");
    }
    return "";
}

// ISO string -> value for <input type="datetime-local">
function toLocalInput(iso) {
    if (!iso) return "";
    const d = new Date(iso);
    if (isNaN(d)) return "";
    const pad = (n) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

// datetime-local value -> ISO string (or null)
function fromLocalInput(value) {
    if (!value) return null;
    const d = new Date(value);
    return isNaN(d) ? null : d.toISOString();
}
